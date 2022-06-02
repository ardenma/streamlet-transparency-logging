# A Transparency Logging System for Streamlet

To run Streamlet: 
- Open N terminal instances, where N=the number of Streamlet nodes you wish to run.
- On each, run: "cargo run N h1", "cargo run N h2", ..., etc. The first argument is the number of nodes, and the second argument is a unique name assigned to that node and used for leader election. 
- In order to view all Streamlet data (messages received, blocks proposed, etc.), run with RUST_LOG=info
- On one of the Streamlet nodes, type "init". Once all nodes have printed, "\[name\] is done with initialization; has \[N\] peer(s); starting epoch timer", type "end init" (or "e i") into one of the Streamlet nodes. This will complete and close the initialization process, start the epoch counter, and kick off the Streamlet protocol. 

For the application: 
- On one terminal, type: "cargo run app". This starts the application. We recommend running with RUST_LOG=info to view data.
- To send data to Streamlet, type any key into the terminal running the application and press "enter". You may wish to do this multiple times consecutively in order to ensure that consecutive epochs are achievable. 
- To request the latest finalized block from Streamlet, type "request block" and press enter. 
- To request the entire finalized chain, type "request chain" and press enter. 
