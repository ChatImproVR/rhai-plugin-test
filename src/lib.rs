// Written by new.py, with love
use cimvr_engine_interface::{make_app_state, prelude::*, println};
use rhai::Engine;

// All state associated with client-side behaviour
struct ClientState;

impl UserState for ClientState {
    // Implement a constructor
    fn new(_io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        let mut engine = Engine::new();
        let res = engine.eval::<i64>("68 + 1").unwrap();
        println!("{}", res);

        Self
    }
}

// All state associated with server-side behaviour
struct ServerState;

impl UserState for ServerState {
    // Implement a constructor
    fn new(_io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        Self
    }
}

// Defines entry points for the engine to hook into.
// Calls new() for the appropriate state.
make_app_state!(ClientState, ServerState);
