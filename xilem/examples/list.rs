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

use rand::Rng;
use xilem::{button, v_stack, App, AppLauncher, ForEach, IdPrinter, View};

#[derive(Clone, Debug)]
struct Item {
    key: usize,
    name: String,
}

#[derive(Default)]
struct AppData {
    next_key: usize,
    name: usize,
    list: Vec<Item>,
}

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    v_stack((
        format!("Next name: {}", data.name),
        button("Increase", |data: &mut AppData| {
            data.name = data.name.wrapping_add(1);
        }),
        button("Decrease", |data: &mut AppData| {
            data.name = data.name.wrapping_sub(1);
        }),
        button("Add", |data: &mut AppData| {
            data.list.push(Item {
                key: data.next_key,
                name: data.name.to_string(),
            });
            data.next_key += 1;
        }),
        button("Shuffle", |data: &mut AppData| {
            let len = data.list.len();
            if len >= 2 {
                let mut rng = rand::thread_rng();
                for i in 0..(len - 1) {
                    data.list.swap(i, rng.gen_range(i..len));
                }
            }
        }),
        ForEach::new(
            data.list.clone(),
            |i: &Item| i.key,
            |data: &mut AppData, i: &Item| {
                let key = i.key;
                // For each item
                v_stack((
                    "=======".to_owned(),
                    IdPrinter,
                    format!("Item {} - Next {}", i.name, data.name),
                    button("Remove", move |data: &mut AppData| {
                        data.list.retain(|e| e.key != key)
                    }),
                ))
            },
        ),
    ))
}

pub fn main() {
    let app = App::new(
        AppData {
            next_key: 0,
            name: 0,
            list: vec![],
        },
        app_logic,
    );
    AppLauncher::new(app).run();
}
