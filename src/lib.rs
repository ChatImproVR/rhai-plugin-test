// Written by new.py, with love
use cimvr_engine_interface::{make_app_state, prelude::*, println};

use cimvr_common::ui::{Schema, State, UiHandle, UiStateHelper, UiUpdate};

// All state associated with client-side behaviour
struct ClientState {
    ui: UiStateHelper,
    rhai_engine: rhai::Engine,
    rhai_scope: rhai::Scope<'static>,
    widget: UiHandle,
}

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        let rhai_engine = rhai::Engine::new();

        let mut ui = UiStateHelper::new();

        // Create chat "window"
        let schema = vec![
            Schema::TextInput,
            Schema::Button { text: "Run".into() },
            Schema::Label,
        ];
        let state = vec![
            State::TextInput { text: "".into() },
            State::Button { clicked: false },
            State::Label { text: "".into() },
        ];
        let widget = ui.add(io, "Rhai", schema, state);

        sched
            .add_system(Self::ui_update)
            .subscribe::<UiUpdate>()
            .build();

        Self {
            rhai_engine,
            rhai_scope: rhai::Scope::new(),
            widget,
            ui,
        }
    }
}

impl ClientState {
    fn ui_update(&mut self, io: &mut EngineIo, _query: &mut QueryResult) {
        // Update the UI helper's internal state
        self.ui.download(io);

        // Check for UI updates
        if io.inbox::<UiUpdate>().next().is_some() {
            // Read the text input
            let ui_state = self.ui.read(self.widget);
            let State::TextInput { text } = &ui_state[0] else { panic!() };

            if let State::Button { clicked: true } = ui_state[1] {
                let result = self
                    .rhai_engine
                    .eval_with_scope::<rhai::Dynamic>(&mut self.rhai_scope, text);

                let result_text = match result {
                    Ok(dyn_val) => dyn_val.to_string(),
                    Err(e) => format!("Error: {:#}", e),
                };

                // Clear the text input
                self.ui.modify(io, self.widget, |states| {
                    states[2] = State::Label { text: result_text.clone() };
                });
            }
        }
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
