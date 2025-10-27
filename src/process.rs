mod engine;
pub mod handler;
mod scaffold;

use crate::{
    api::{Data, IntermediateEvent, TaskResult, With},
    bpmn::Bpmn,
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

#[cfg(feature = "async")]
use std::future::Future;

#[cfg(feature = "async")]
fn run_async_task<Fut>(fut: Fut) -> TaskResult
where
    Fut: Future<Output = TaskResult> + 'static + Send,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(|rt| rt.block_on(fut))
        .unwrap_or_else(|e| {
            log::error!("Failed to create tokio runtime: {}", e);
            None
        })
}

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
        F: Fn(Data<T>) -> TaskResult + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Task(Box::new(func)));
        self
    }

    #[cfg(feature = "async")]
    pub fn task_async<F, Fut>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Fut + 'static + Sync + Send,
        Fut: Future<Output = TaskResult> + 'static + Send,
    {
        self.handler.add_callback(
            name,
            Callback::Task(Box::new(move |data| {
                let fut = func(data);

                // If we're inside a runtime, spawn a thread to avoid nested block_on
                if tokio::runtime::Handle::try_current().is_ok() {
                    std::thread::spawn(move || run_async_task(fut))
                        .join()
                        .unwrap_or_else(|e| {
                            log::error!("Thread panicked: {:?}", e);
                            None
                        })
                } else {
                    // No runtime exists, create one and block on the future
                    run_async_task(fut)
                }
            })),
        );
        self
    }

    /// Register an exclusive gateway function with name or bpmn id
    pub fn exclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Exclusive(Box::new(func)));
        self
    }

    /// Register an inclusive gateway function with name or bpmn id
    pub fn inclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> With + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Inclusive(Box::new(func)));
        self
    }

    /// Register an event based gateway function with name or bpmn id
    pub fn event_based<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> IntermediateEvent + 'static + Sync + Send,
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
    /// Run the process and return the `T` or an `Error`.
    /// ```
    /// use snurr::Process;
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
    ///             input.lock().unwrap().count += 1;
    ///             None
    ///         })
    ///         .exclusive("equal to 3", |input| {
    ///             match input.lock().unwrap().count {
    ///                 3 => "YES",
    ///                 _ => "NO",
    ///             }
    ///             .into()
    ///         })
    ///         .build()?;
    ///
    ///     // Run the process with input data
    ///     let counter = bpmn.run(Counter::default())?;
    ///
    ///     // Print the result.
    ///     println!("Count: {}", counter.count);
    ///     Ok(())
    /// }
    /// ```
    pub fn run(&self, data: T) -> Result<T, Error>
    where
        T: Send,
    {
        let data = Arc::new(Mutex::new(data));

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
                self.execute(ExecuteInput::new(process_data, Arc::clone(&data)))?;
            }
        }

        Arc::into_inner(data)
            .ok_or(Error::NoProcessResult)?
            .into_inner()
            .map_err(|_| Error::NoProcessResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?
            .task("Count 1", |_| None)
            .exclusive("equal to 3", |_| None)
            .build()?;
        bpmn.run({})?;
        Ok(())
    }
}
