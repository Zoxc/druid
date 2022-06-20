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

use std::sync::Arc;

use xilem::{button, v_stack, Adapt, App, AppLauncher, LayoutObserver, LocalState, Memoize, View};

fn counter(name: &'static str, outer: &mut u32, count: &mut u32) -> impl View<u32> {
    button(
        format!("{} counter: {} - Outer value: {}", name, *count, *outer),
        |data| *data += 1,
    )
}

fn app_logic(data: &mut u32) -> impl View<u32> {
    let outer = button(format!("Outer counter: {}", data), |data| *data += 1);
    let local_1 = LocalState::new(
        || 0,
        |outer: &mut u32, inner: &mut u32| counter("Child 1", outer, inner),
    );
    let local_2 = LocalState::new(
        || 0,
        |outer: &mut u32, inner: &mut u32| counter("Child 2", outer, inner),
    );
    v_stack((outer, " ".to_owned(), local_1, " ".to_owned(), local_2))
}

pub fn main() {
    let app = App::new(u32::default(), app_logic);
    AppLauncher::new(app).run();
}
