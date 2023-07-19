use std::collections::HashMap;

// Written by new.py, with love
use cimvr_engine_interface::{dbg, make_app_state, pkg_namespace, prelude::*, println};

use cimvr_common::{
    render::Render,
    ui::{
        egui::{Color32, Label, RichText, ScrollArea, TextEdit, TextStyle},
        GuiInputMessage, GuiTab,
    },
    Transform,
};
use rhai::{Dynamic, AST};
use syntax_highlighting::code_view_ui;
mod syntax_highlighting;

// All state associated with client-side behaviour
struct ClientState {
    engine: rhai::Engine,
    scope: rhai::Scope<'static>,
    editor: GuiTab,
    console: GuiTab,
    script: String,
    response_text: String,
    command_line: String,
    //response_is_error: bool,
    command: Option<String>,
}

const BUILTIN_SCRIPT: &str = include_str!("builtins.rhai");
const DEFAULT_SCRIPT: &str = include_str!("default.rhai");

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        let mut rhai_engine = rhai::Engine::new();
        rhai_engine.on_print(|s: &str| println!("{}", s));

        sched
            .add_system(Self::ui_update)
            .subscribe::<GuiInputMessage>()
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

        let editor = GuiTab::new(io, pkg_namespace!("editor"));
        let console = GuiTab::new(io, pkg_namespace!("console"));

        Self {
            //response_is_error: false,
            command: None,
            command_line: "state.run_me()".into(),
            engine: rhai_engine,
            scope: rhai_scope,
            editor,
            console,
            script: DEFAULT_SCRIPT.to_string(),
            response_text: "".into(),
        }
    }
}

impl ClientState {
    fn run_command(&mut self, command: &str) -> Result<Dynamic, String> {
        // Run update() function in script
        //println!("{}", self.scope);
        let script = format!("\n{}\n{}\n{}", self.script, BUILTIN_SCRIPT, command);
        let result = self
            .engine
            .eval_with_scope::<Dynamic>(&mut self.scope, &script);

        match result {
            Err(e) => {
                self.response_text = format!("error running {}: {:#}", command, e);
                Err(e.to_string())
            }
            Ok(dy) => Ok(dy),
        }
    }

    fn transform_editor(&mut self, _io: &mut EngineIo, query: &mut QueryResult) {
        // The variable "State" will always be available
        if self.scope.get("state").is_none() {
            self.scope.push("state", rhai::Map::new());
        }

        // Copy ECS data into rhai
        let map: HashMap<String, Transform> = query
            .iter("Transforms")
            .map(|id @ EntityId(num)| (num.to_string(), query.read::<Transform>(id)))
            .collect();
        let transforms_rhai = rhai::serde::to_dynamic(&map).unwrap();

        // TODO: Just how slow is this?
        if let Some(mut state) = self.scope.remove::<rhai::Map>("state") {
            state.insert("transforms".into(), transforms_rhai);
            self.scope.set_value("state", state);
        }

        // Run update() function in script
        //println!("{}", self.scope);
        let _ = self.run_command("state.update();");

        // Run any command line commands
        if let Some(command) = self.command.take() {
            if let Ok(d) = self.run_command(&command) {
                self.response_text = format!("Returned: {}", d);
            }
        }

        // Copy ECS data back into cimvr
        if let Some(mut state) = self.scope.remove::<rhai::Map>("state") {
            if let Some(transforms) = state.remove("transforms".into()) {
                let ret_map: Result<HashMap<String, Transform>, _> =
                    rhai::serde::from_dynamic(&transforms);

                match ret_map {
                    Err(e) => self.response_text = format!("error: {}", e),
                    Ok(ret_map) => {
                        for (key, value) in ret_map {
                            let ent = EntityId(key.parse().unwrap());
                            query.write(ent, &value);
                        }
                    }
                }
            }
            self.scope.set_value("state", state);
        }
    }

    fn ui_update(&mut self, io: &mut EngineIo, _query: &mut QueryResult) {
        self.editor.show(io, |ui| {
            /*
            let code_changed = ui
                .add_sized(
                    ui.available_size(),
                    TextEdit::multiline(&mut self.script)
                        .code_editor(),
                )
                .changed();
            */

            let code_changed = ScrollArea::vertical().show(ui, |ui| {
                code_view_ui(ui, &mut self.script)
            });

            if code_changed.inner {
                if let Err(e) = self.engine.compile(&self.script) {
                    self.response_text = format!("Script compile error: {:#}", e);
                    //self.response_is_error = true;
                } else {
                    self.response_text = format!("Compilation successful");
                    //self.response_is_error = false;
                }
            }
        });

        self.console.show(io, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.command_line);
                if ui.button("Run").clicked() {
                    self.command = Some(self.command_line.clone());
                }
            });

            if self.response_text.contains("error") {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.label(
                        RichText::new(&self.response_text)
                            .color(Color32::RED)
                            .monospace(),
                    );
                });
            } else if self.response_text.contains("Returned") {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(&self.response_text).monospace());
                });
            } else {
                ui.add_sized(ui.available_size(), Label::new(&self.response_text));
            }
        });

        /*
        // Compile the script
        let ui_state = self.ui.read(self.widget);

        // Check for UI updates
        if io.inbox::<UiUpdate>().next().is_some() {
            let State::TextBox { text } = &ui_state[4] else { panic!() };
            let script_compile_result = self.engine.compile(text);

            match script_compile_result {
                Ok(_ast) => {
                    self.script = text.clone();
                    if self.response_text.contains("Script compile error") {
                        self.response_text = format!("Compilation successful");
                    }
                }
                Err(e) => self.response_text = format!("Script compile error: {:#}", e),
            };
        }

        // Set the command line
        if ui_state[1] == (State::Button { clicked: true })
            || ui_state[2] == (State::CheckBox { checked: true })
        {
            let State::TextInput { text } = &ui_state[0] else { panic!() };
            //let cmd_compile_result = self.engine.compile_expression(text);
            self.command = Some(text.clone());
        }

        // Set the response text
        self.ui.modify(io, self.widget, |ui_state| {
            ui_state[3] = State::Label {
                text: self.response_text.clone(),
            };
        });
        */
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

/*
 *fn init() {
    return #{
        "queries": #{
            "transforms": "cimvr_common/Transform",
        },
        "subscriptions": [
            "cimvr_common/KeyboardInput"
        ],
    };
}
*/
