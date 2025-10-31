mod engine;
pub mod handler;
mod scaffold;

use crate::{
    api::{Data, EndNode, IntermediateEvent, ProcessOutput, TaskResult, With},
    bpmn::{Bpmn, Symbol},
    diagram::{Diagram, reader::read_bpmn},
    error::Error,
    process::handler::Callback,
};
use engine::ExecuteInput;
use handler::Handler;
use std::{
    marker::PhantomData,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

/// Process that contains information from the BPMN file
pub struct Process<T, S = Build>
where
    Self: Sync + Send,
{
    diagram: Diagram,
    handler: Handler<T>,
    _marker: PhantomData<S>,
}

/// Process Build state
pub struct Build;

/// Process Run state
pub struct Run;

impl<T> Process<T> {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            diagram: read_bpmn(quick_xml::Reader::from_file(path)?)?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }

    /// Register a task function with name or bpmn id
    pub fn task<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Result<TaskResult, Error> + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Task(Box::new(func)));
        self
    }

    /// Register an exclusive gateway function with name or bpmn id
    pub fn exclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Result<Option<&'static str>, Error> + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Exclusive(Box::new(func)));
        self
    }

    /// Register an inclusive gateway function with name or bpmn id
    pub fn inclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Result<With, Error> + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Inclusive(Box::new(func)));
        self
    }

    /// Register an event based gateway function with name or bpmn id
    pub fn event_based<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Result<IntermediateEvent, Error> + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::EventBased(Box::new(func)));
        self
    }

    /// Install and check that all required functions have been registered. You cannot run a process before `build` is called.
    /// If `build` returns an error, it contains the missing functions.
    pub fn build(mut self) -> Result<Process<T, Run>, Error> {
        let result = self.diagram.install_and_check(self.handler.build()?);
        if result.is_empty() {
            Ok(Process {
                diagram: self.diagram,
                handler: self.handler,
                _marker: Default::default(),
            })
        } else {
            Err(Error::MissingImplementations(
                result.into_iter().collect::<Vec<_>>().join(", "),
            ))
        }
    }
}

impl<T> FromStr for Process<T> {
    type Err = Error;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<T> Process<T, Run> {
    /// Run the process and return the `ProcessOutput<T>` containing the final data and end node information, or an `Error`.
    ///
    /// Registered functions can return `Err(Error)` to stop execution immediately.
    ///
    /// ```
    /// use snurr::{Process, Error};
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     pretty_env_logger::init();
    ///
    ///     // Create process from BPMN file
    ///     let bpmn = Process::<Counter>::new("examples/example.bpmn")?
    ///         .task("Count 1", |input| {
    ///             let mut data = input.lock().unwrap();
    ///             if data.count > 100 {
    ///                 return Err(Error::ProcessBreak("Count too high".into()));
    ///             }
    ///             data.count += 1;
    ///             Ok(None)
    ///         })
    ///         .exclusive("equal to 3", |input| {
    ///             match input.lock().unwrap().count {
    ///                 3 => Ok(Some("YES")),
    ///                 _ => Ok(Some("NO")),
    ///             }
    ///         })
    ///         .build()?;
    ///
    ///     // Run the process with input data
    ///     let result = bpmn.run(Counter::default())?;
    ///
    ///     // Print the result.
    ///     println!("Count: {}", result.data.count);
    ///     println!("Ended at: {}", result.end_node.id);
    ///     Ok(())
    /// }
    /// ```
    pub fn run(&self, data: T) -> Result<ProcessOutput<T>, Error>
    where
        T: Send,
    {
        let data = Arc::new(Mutex::new(data));
        let mut end_node_name = None;
        let mut end_node_id = String::new();
        let mut end_event_symbol = Symbol::None;

        // Run every process specified in the diagram
        for bpmn in self
            .diagram
            .get_definition()
            .ok_or(Error::MissingDefinitionsId)?
            .iter()
        {
            if let Bpmn::Process {
                id,
                data_index: Some(index),
                ..
            } = bpmn
            {
                let process_data = self
                    .diagram
                    .get_process(*index)
                    .ok_or_else(|| Error::MissingProcessData(id.bpmn().into()))?;
                let end_event = self.execute(ExecuteInput::new(process_data, Arc::clone(&data)))?;
                end_node_name = end_event.name.clone();
                end_node_id = end_event.id.bpmn().to_string();
                end_event_symbol = end_event.symbol.clone().unwrap_or(Symbol::None);
            }
        }

        let data = Arc::into_inner(data)
            .ok_or(Error::NoProcessResult)?
            .into_inner()
            .map_err(|_| Error::NoProcessResult)?;

        Ok(ProcessOutput {
            data,
            end_node: EndNode {
                id: end_node_id,
                name: end_node_name,
                symbol: end_event_symbol,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?
            .task("Count 1", |_| Ok(None))
            .exclusive("equal to 3", |_| Ok(None))
            .build()?;
        let _result = bpmn.run({})?;
        Ok(())
    }
}
