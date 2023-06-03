use std::collections::HashMap;

// Written by new.py, with love
use cimvr_engine_interface::{dbg, make_app_state, prelude::*, println};

use cimvr_common::{
    render::Render,
    ui::{Schema, State, UiHandle, UiStateHelper, UiUpdate},
    Transform,
};
use rhai::{Dynamic, AST};

// All state associated with client-side behaviour
struct ClientState {
    ui: UiStateHelper,
    rhai_engine: rhai::Engine,
    rhai_scope: rhai::Scope<'static>,
    widget: UiHandle,
    current_ast: AST,
    response_text: String,
}

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        let mut rhai_engine = rhai::Engine::new();
        rhai_engine.on_print(|s: &str| println!("{}", s));

        let mut ui = UiStateHelper::new();

        // Create chat "window"
        let schema = vec![
            Schema::TextBox,
            //Schema::Button { text: "Run".into() },
            Schema::Label,
        ];
        let state = vec![
            State::TextBox { text: "".into() },
            //State::Button { clicked: false },
            State::Label { text: "".into() },
        ];
        let widget = ui.add(io, "Rhai", schema, state);

        sched
            .add_system(Self::transform_editor)
            .query(
                "Transforms",
                Query::new()
                    .intersect::<Transform>(Access::Write)
                    .intersect::<Render>(Access::Read),
            )
            .build();

        sched
            .add_system(Self::ui_update)
            .subscribe::<UiUpdate>()
            .build();

        let rhai_scope = rhai::Scope::new();

        Self {
            rhai_engine,
            rhai_scope,
            widget,
            ui,
            current_ast: AST::default(),
            response_text: "".into(),
        }
    }
}

impl ClientState {
    fn transform_editor(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        let map: HashMap<String, Transform> = query
            .iter("Transforms")
            .map(|id @ EntityId(num)| (num.to_string(), query.read::<Transform>(id)))
            .collect();

        let rhai_dyn_map = rhai::serde::to_dynamic(&map).unwrap();

        self.rhai_scope.push_dynamic("transforms", rhai_dyn_map);

        let result = self.rhai_engine
            .run_ast_with_scope(&mut self.rhai_scope, &self.current_ast);

        if let Err(e) = result {
            self.response_text = format!("Runtime error: {:#}", e);
        }
    }

    fn ui_update(&mut self, io: &mut EngineIo, _query: &mut QueryResult) {
        // Update the UI helper's internal state
        self.ui.download(io);

        // Check for UI updates
        if io.inbox::<UiUpdate>().next().is_some() {
            // Read the text input
            let ui_state = self.ui.read(self.widget);
            let State::TextBox { text } = &ui_state[0] else { panic!() };
            let result = self.rhai_engine.compile(text);

            self.response_text = match result {
                Ok(ast) => {
                    self.current_ast = ast;
                    format!("Compilation successful")
                }
                Err(e) => format!("Compile error: {:#}", e),
            };
        }

        // Clear the text input
        self.ui.modify(io, self.widget, |states| {
            states[1] = State::Label {
                text: self.response_text.clone(),
            };
        });
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
