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
    engine: rhai::Engine,
    scope: rhai::Scope<'static>,
    widget: UiHandle,
    script_ast: AST,
    response_text: String,
    command: Option<String>,
}

const DEFAULT_SCRIPT: &str = r#"fn update() {
    
}

fn run_me() {
    print("Hello, world!");
}
"#;

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        let mut rhai_engine = rhai::Engine::new();
        rhai_engine.on_print(|s: &str| println!("{}", s));

        let mut ui = UiStateHelper::new();

        // Create chat "window"
        let schema = vec![
            Schema::TextBox,
            Schema::TextInput,
            Schema::Button { text: "Run".into() },
            Schema::Label,
        ];
        let state = vec![
            State::TextBox {
                text: DEFAULT_SCRIPT.into(),
            },
            State::TextInput {
                text: "run_me()".into(),
            },
            State::Button { clicked: false },
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
            command: None,
            engine: rhai_engine,
            scope: rhai_scope,
            widget,
            ui,
            script_ast: AST::default(),
            response_text: "".into(),
        }
    }
}

impl ClientState {
    fn transform_editor(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        // Copy ECS data into rhai
        let map: HashMap<String, Transform> = query
            .iter("Transforms")
            .map(|id @ EntityId(num)| (num.to_string(), query.read::<Transform>(id)))
            .collect();

        let rhai_dyn_map = rhai::serde::to_dynamic(&map).unwrap();

        self.scope.push_dynamic("transforms", rhai_dyn_map);

        // Run update() function in script
        let result = self
            .engine
            .call_fn::<()>(&mut self.scope, &self.script_ast, "update", ());

        if let Err(e) = result {
            self.response_text = format!("Error running update(): {:#}", e);
        }

        // Run any command line commands
        if let Some(command) = self.command.take() {
            let result = self
                .engine
                .eval_ast_with_scope::<()>(&mut self.scope, &self.script_ast);

            if let Err(e) = result {
                self.response_text = format!("{}", e);
            } else {
                let result = self
                    .engine
                    .eval_with_scope::<Dynamic>(&mut self.scope, &command);

                self.response_text = match result {
                    Ok(result) => format!("Command: {}", result),
                    Err(e) => format!("Command caused error: {}", e),
                };
                dbg!(&self.response_text);
            }
        }

        // Copy ECS data back into cimvr
        let returned_map = self.scope.get("transforms").unwrap();
        let ret_map: HashMap<String, Transform> = rhai::serde::from_dynamic(returned_map).unwrap();
        for (key, value) in ret_map {
            let ent = EntityId(key.parse().unwrap());
            query.write(ent, &value);
        }
    }

    fn ui_update(&mut self, io: &mut EngineIo, _query: &mut QueryResult) {
        // Update the UI helper's internal state
        self.ui.download(io);

        // Check for UI updates
        if io.inbox::<UiUpdate>().next().is_some() {
            // Compile the script
            let ui_state = self.ui.read(self.widget);
            let State::TextBox { text } = &ui_state[0] else { panic!() };
            let script_compile_result = self.engine.compile(text);

            self.response_text = match script_compile_result {
                Ok(ast) => {
                    self.script_ast = ast;
                    format!("Compilation successful")
                }
                Err(e) => format!("Compile error: {:#}", e),
            };

            // Set the command line
            if ui_state[2] == (State::Button { clicked: true }) {
                let State::TextInput { text } = &ui_state[1] else { panic!() };
                //let cmd_compile_result = self.engine.compile_expression(text);
                self.command = Some(text.clone());

                /*
                   match cmd_compile_result {
                   Ok(ast) => {
                   }
                   Err(e) => self.response_text = format!("Commandline error: {:#}", e),
                   };
                   */
            }
        }

        // Set the response text
        self.ui.modify(io, self.widget, |states| {
            states[3] = State::Label {
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
