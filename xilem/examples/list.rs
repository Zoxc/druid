// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use xilem::{button, v_stack, Adapt, App, AppLauncher, ForEach, LayoutObserver, Memoize, View};

#[derive(Default)]
struct AppData {
    list: Vec<String>,
}

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    v_stack((
        button("Add", |data: &mut AppData| {
            data.list.push("Test".to_string())
        }),
        ForEach::new(
            data.list.clone(),
            |i: &String| i.len(),
            |i: &String| {
                let i = i.to_owned();
                // For each item
                v_stack((
                    i.clone(),
                    button("Remove", move |data: &mut AppData| {
                        data.list.retain(|e| e != &i)
                    }),
                ))
            },
        ),
    ))
}

pub fn main() {
    let app = App::new(
        AppData {
            list: vec!["Hi".to_string(), "There".to_string()],
        },
        app_logic,
    );
    AppLauncher::new(app).run();
}
