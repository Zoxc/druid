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

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use sha2::{Digest, Sha256};
use xilem::{button, list, loader, scroll_view, v_stack, App, AppLauncher, Memoize, View};

fn compute_hash(salt: u64, i: usize, abort: Arc<AtomicBool>) -> Option<String> {
    let mut s = format!("{}.{}", salt, i);
    for _ in 0..i {
        if abort.load(Ordering::Acquire) {
            return None;
        }
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        s = hex::encode(result);
    }
    Some(s)
}

#[derive(PartialEq, Clone, Copy)]
struct HashSetting {
    fast: bool,
    salt: u64,
}

struct Demo {
    settings: HashSetting,
    rt: Arc<tokio::runtime::Runtime>,
}

struct OnDrop<F: Fn()>(pub F);

impl<F: Fn()> Drop for OnDrop<F> {
    #[inline]
    fn drop(&mut self) {
        (self.0)();
    }
}

fn list_item(i: usize, settings: HashSetting, rt: Arc<tokio::runtime::Runtime>) -> impl View<Demo> {
    Memoize::new(settings, move |settings| {
        let settings = *settings;
        let rt = rt.clone();
        loader(format!("{}: Calculating...", i), move || {
            let rt = rt.clone();
            async move {
                // Create a handle which we can use to abort.
                let aborted = Arc::new(AtomicBool::new(false));
                let aborted_ = aborted.clone();
                // Trigger it if the future is dropped.
                let _abort = OnDrop(|| aborted_.store(true, Ordering::Release));

                let settings = settings;
                rt.spawn(async move {
                    let start = if settings.fast { 1 } else { 5000 };
                    compute_hash(settings.salt, start + i, aborted)
                        .map(|hash| format!("{}: {}", i, hash))
                        .unwrap_or_default()
                })
                .await
                .unwrap()
            }
        })
    })
}

fn app_logic(demo: &mut Demo) -> impl View<Demo> {
    let rt = demo.rt.clone();
    let settings = demo.settings;
    v_stack((
        format!("Salt: {}", demo.settings.salt),
        button("Increase", |demo: &mut Demo| demo.settings.salt += 1),
        " ".to_string(),
        format!(
            "Iterations: {}",
            if demo.settings.fast { "Low" } else { "High" }
        ),
        button("Toggle", |demo: &mut Demo| {
            demo.settings.fast = !demo.settings.fast
        }),
        " ".to_string(),
        scroll_view(list(10_000, 16.0, move |i| {
            list_item(i, settings, rt.clone())
        })),
    ))
}

fn main() {
    // Create a separate runtime for background computation
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap(),
    );

    let demo = Demo {
        settings: HashSetting {
            fast: false,
            salt: 0,
        },
        rt: rt.clone(),
    };

    let app = App::new(demo, app_logic);
    AppLauncher::new(app).run();
}
