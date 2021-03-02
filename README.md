# Node Crunch
Allows to distribute computations across several nodes.
(Keywords: numeric computing, scientific computing, HPC)

## Table of contents
- [Introduction](#introduction)
- [Server example](#server_example)
- [Node example](#node_example)
- [How to start the application](#start_application)
- [Full working examples](#full_examples)
- [How does it compare to *x* ?](#compare_to)
- [TODO](#todo)
- [FAQ](#faq)
- [License](#license)


## Introduction <a name="introduction"></a>

**Note:** *it is still in development and the API may change.*
**Note 2:** *the communication between the server and the nodes are not encrypted, so use this in a trused enviroment only!*

Node Crunch is a crate that allows users to write distributed code easily. The code is organized in a two main parts: one for the server and one for the node.

This is refelected in the two traits that must be implemented for Node Crunch to work:

1. The **NCSever** trait. This contains the functionality for the server. Here the data is split up for the nodes to do the computation and later the data is collected from all the nodes. The trait has five functions that have to be implemented accordingly:

    1.1 `initial_data()`
    This is called when a node contacts the server for the first time. Here the server internally assigns a new and unique node id and sends that back to the node together with an optional initial data. This only needs to be impelented when initial data has to be sent to each node before the main computation begins.

    1.2 `prepare_data_for_node()`
    After initializing the node it contacts the server again for some data to process. This function has to be implemented to prepare the data that will be send to the node.

    1.3 `process_data_from_node()`
    When the node is done with processing the data it will send the results back to the server. This function has to be implemented by the server code.

    1.4 `heartbeat_timeout()`
    Each node sends a heartbeat message internally to the server. If one of the node fails in sending this heartbeat message then this function is called. It has to be implemented in the server code and here the node should be marked as offline.

    1.5 `finish_job()`
    After all the data has been processed this function is called. Implement this function for example to write all the results to disk, etc.

2. The **NCNode** trait. This contains the functionality for each node. Here the most of the processing is done by the node code. The trait has two methods that have to be implemented accordingly.

    2.1 `set_initial_data()` When the node contacts the server for the first time the new node id is given to the node here along with some optional initial data. This function only has to be implemented when there is a need for some initial data.

    2.2 `process_data_from_server()` Here the main processing is done by the node code. This function receives the data that has to be processed and returns the processed data.

Both traits will be impelented for the user defined data structures that are passed to the starter for the server and the starter for the node.

## Server example <a name="server_example"></a>

Here is a small example for the server:

```rust
struct MyServer {
    // ...
}

impl NCServer for MyStruct {
    // The function initial_data() doesn't have to be impelented if no initial data is needed.

    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus, NCError> {
        // ...
    }

    fn process_data_from_node(&mut self, node_id: NodeID, node_data: &[u8]) -> Result<(), NCError> {
        // ...
    }

    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>) {
        // ...
    }

    fn finish_job(&mut self) {
        // ...
    }
}

let server = MyServer {
    // ...
}

// The configuration is shared between server and node.
let configuration = NCConfiguration {
    // ...
    ..Default::default()
};

let mut server_starter = NCServerStarter::new(configuration);

match server_starter.start(server) {
    Ok(_) => {
        info!("Calculation finished");
    }
    Err(e) => {
        error!("An error occurred: {}", e);
    }
}

```

## Node example <a name="node_example"></a>

Here is a small example for the node:

```rust
struct MyNode {
    // ...
}

impl NCNode for MyNode {
    // The function set_initial_data() doesn't have to be impelented if no initial data is needed.

    fn process_data_from_server(&mut self, data: &[u8]) -> Result<Vec<u8>, NCError> {
        // ...
    }
}

// The configuration is shared between server and node.
let configuration = NCConfiguration {
    // ...
    ..Default::default()
};

let node = MyNode{};
let mut node_starter = NCNodeStarter::new(configuration);

match node_starter.start(node) {
    Ok(_) => {
        info!("Calculation finished");
    }
    Err(e) => {
        error!("An error occurred: {}", e);
    }
}

```

## How to start the application <a name="start_application"></a>

Usually there is one binary for both the server and the node code. You just specify which mode is used for example via the command line. Since there is only one server and lots of nodes, the node mode should be the default:

```bash
./myapp -s & # runs in server mode

./myapp & # start one task in node mode

./myapp & # start another taks in node mode

```

And in the main function a simple check does the job:

```rust
fn main() {
    // Read settings from command line or configuration file
    let options = get_option();

    if options.server {
        run_server(options);
    } else {
        run_node(options);
    }
}
```

## Full working examples <a name="full_examples"></a>

It looks complicatd at first but there are a couple of examples that show how to use it in "real world" applications:

- Distributed Mandelbrot
- Distributed Path Tracing
- ...

## How does it compare to *x* ? <a name="compare_to"></a>

### MPI

[MPI](https://www.open-mpi.org/) (Message Passing Interface) is the gold standard for distributed computing. It's battle tested, highly optimized and has support for C, C++ and Fortran. There are also bindings for other programming languages available.
Node Crunsh only works with Rust and is still in development.
Writing correct code in MPI is difficult and if one of the nodes crash all other nodes (including the main node) will be terminated. Node Crunsh on the other hand doesn't care if one of the nodes crash. The heartbeat system detect if one of the nodes is no longer available and the server continues with the other nodes. Rust makes it very easy to write correct code, the user doesn't have to take care of synchronisation and other things and can fully concentrate on the computing part.
The number of nodes in MPI is fixed the whole time once the application has started, whereas with Node Crunsh you can add more and more nodes while it's running if needed.

### BOINC

[BOINC](https://boinc.berkeley.edu/) (Berkeley Open Infrastructure for Network Computing) is also used for distributed computing and is well tested and highly performant. Scientists use it to scale their computations massively by providing desktop clients that can run as screen savers on normal PCs, Macs, etc. It utilizes the resources that the computer is currently not using for numeric computations. The most famous (and first) application is the SETI@home client.
Node Crunsh is a lot smaller but also a lot easier to set up ;-)


## TODO <a name="todo"></a>

You may have noticed that the data that is send around is passed as `&[u8]` or `Vec<u8>`. In the future this will be generic parameters that correspond to user defined data structures. For now the user has to de- / serialize all the data that is passed to and returned from the trait functions. But helper functions are provided in the `nc_utils` module.

Other things that need to be done in the next releases:

- Currently the communitaion between the server and the nodes are not compressed, not encrypted and there is no authentication. Maybe I'll switch to a small web framework that does all this, like [warp](https://github.com/seanmonstar/warp) for example. A web framework may be overkill but it would be very easy to add a nice web GUI for the server.

- The crate doesn't use any async code ([tokio](https://github.com/tokio-rs/tokio), [async-std](https://github.com/async-rs/async-std), ...). Computations usually are more CPU bound than IO bound but there are always exceptions to this. So the currently used thread based strategy will be the limiting factor in some cases.

- More helper functions and data structures have to be provided. These will show up when there are more use cases and I'm happy to include them when they make sense.

- And of course refactoring and adding more and better documentation is always good ;-)

## FAQ <a name="faq"></a>

- Where does the name come from ? Compute *node* and number *crunshing*.
- Will you add feature *x* ? It depends, if it makes sense and helps other users as well.
- Can I use [Rayon](https://github.com/rayon-rs/rayon) and / or GPGPU with Node Crunsh ? Yes of course, no problem.

## License <a name="license"></a>

This crate is licensed under the MIT license.
