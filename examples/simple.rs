use snurr::Process;

extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = Process::<Counter>::new("examples/example.bpmn")?
        .task("Count 1", |input| {
            input.lock().unwrap().count += 1;
            Ok(None)
        })
        .exclusive("equal to 3", |input| {
            match input.lock().unwrap().count {
                3 => Ok(Some("YES")),
                _ => Ok(Some("NO")),
            }
        })
        .build()?;

    // Run the process with input data
    let result = bpmn.run(Counter::default())?;

    // Print the result.
    println!("Count: {}", result.data.count);
    println!("Ended at node: {}", result.end_node.id);
    if let Some(name) = &result.end_node.name {
        println!("End node name: {}", name);
    }
    if result.end_node.symbol != snurr::Symbol::None {
        println!("End event type: {:?}", result.end_node.symbol);
    }
    Ok(())
}
