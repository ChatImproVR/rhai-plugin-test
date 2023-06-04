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
    script: String,
    response_text: String,
    command: Option<String>,
}

const DEFAULT_SCRIPT: &str = r#"fn update() {
    if this.x == () {
         this.x = 0;
    }

    this.x += 1;
    print(this.x);
}

fn run_me() {
    print("Hello, world!");
    this.x = 0;
    return this;
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
            .add_system(Self::ui_update)
            .subscribe::<UiUpdate>()
            .build();

        sched
            .add_system(Self::transform_editor)
            .query(
                "Transforms",
                Query::new()
                    .intersect::<Transform>(Access::Write)
                    .intersect::<Render>(Access::Read),
            )
            .build();

        let rhai_scope = rhai::Scope::new();

        Self {
            command: None,
            engine: rhai_engine,
            scope: rhai_scope,
            widget,
            ui,
            script: String::new(),
            response_text: "".into(),
        }
    }
}

impl ClientState {
    fn transform_editor(&mut self, _io: &mut EngineIo, query: &mut QueryResult) {
        // Copy ECS data into rhai
        let map: HashMap<String, Transform> = query
            .iter("Transforms")
            .map(|id @ EntityId(num)| (num.to_string(), query.read::<Transform>(id)))
            .collect();

        let rhai_dyn_map = rhai::serde::to_dynamic(&map).unwrap();

        self.scope.push_dynamic("transforms", rhai_dyn_map);

        // The variable "State" will always be available
        if self.scope.get("state").is_none() {
            self.scope.push("state", rhai::Map::new());
        }

        // Run update() function in script
        //println!("{}", self.scope);
        let update_script = format!("{}\nstate.update();", self.script);
        let result = self
            .engine
            .eval_with_scope::<()>(&mut self.scope, &update_script);

        if let Err(e) = result {
            self.response_text = format!("Error running update(): {:#}", e);
        }

        // Run any command line commands
        if let Some(command) = self.command.take() {
            let cmd_script = format!("{}\nstate.{}", self.script, command);
            let result = self
                .engine
                .eval_with_scope::<Dynamic>(&mut self.scope, &cmd_script);
            match result {
                Err(e) => self.response_text = format!("Error: {}", e),
                Ok(d) => self.response_text = format!("Returned: {}", d),
            }
        }

        // Copy ECS data back into cimvr
        if let Some(returned_map) = self.scope.remove::<Dynamic>("transforms") {
            let ret_map: HashMap<String, Transform> =
                rhai::serde::from_dynamic(&returned_map).unwrap();
            for (key, value) in ret_map {
                let ent = EntityId(key.parse().unwrap());
                query.write(ent, &value);
            }
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

            match script_compile_result {
                Ok(_ast) => {
                    self.script = text.clone();
                    if self.response_text.contains("Error") {
                        self.response_text = format!("Compilation successful");
                    }
                }
                Err(e) => self.response_text = format!("Compile Error: {:#}", e),
            };

            // Set the command line
            if ui_state[2] == (State::Button { clicked: true }) {
                let State::TextInput { text } = &ui_state[1] else { panic!() };
                //let cmd_compile_result = self.engine.compile_expression(text);
                self.command = Some(text.clone());
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
