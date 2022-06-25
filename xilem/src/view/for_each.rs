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

use std::any::Any;

use crate::{
    event::EventResult,
    id::Id,
    widget::{vstack::VStack, Pod},
};

use super::{Cx, View};

pub struct ForEach<I, F, K> {
    items: Vec<I>,
    key: K,
    map: F,
}

pub struct ForEachState<T, A, V: View<T, A>> {
    items: Vec<ItemState<T, A, V>>,
}

struct ItemState<T, A, V: View<T, A>> {
    key: usize,
    id: Id,
    view: V,
    state: V::State,
}

impl<I, F, K> ForEach<I, F, K> {
    pub fn new(items: Vec<I>, key: K, map: F) -> Self {
        ForEach { items, key, map }
    }
}

impl<I, T, A, V, F, K> View<T, A> for ForEach<I, F, K>
where
    V: View<T, A>,
    F: Fn(&mut T, &I) -> V + Send,
    K: Fn(&I) -> usize + Send,
    V::Element: 'static,
    I: Send,
{
    type State = ForEachState<T, A, V>;

    type Element = VStack;

    fn build(&self, cx: &mut Cx, app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let mut children = Vec::new();
            let items = self
                .items
                .iter()
                .map(|item| {
                    let key = (self.key)(item);
                    let view = (self.map)(app_state, item);
                    let (id, state, element) = view.build(cx, app_state);
                    children.push(Pod::new(element));
                    ItemState {
                        key,
                        id,
                        state,
                        view,
                    }
                })
                .collect();

            (ForEachState { items }, VStack::new(children))
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        app_state: &mut T,
    ) -> bool {
        let mut changed = false;
        cx.with_id(*id, |cx| {
            let mut children = Vec::new();
            let mut removed = 0;
            let items = self
                .items
                .iter()
                .map(|item| {
                    let key = (self.key)(item);
                    let view = (self.map)(app_state, item);
                    let index = state
                        .items
                        .iter()
                        .enumerate()
                        .find(|e| e.1.key == key)
                        .map(|e| e.0);
                    match index {
                        Some(index) => {
                            // Reusing an existing view

                            let old_index = index + removed;

                            let mut item = state.items.remove(index);
                            let mut element = element.children_mut().remove(index);
                            removed += 1;
                            let item_changed = view.rebuild(
                                cx,
                                &item.view,
                                &mut item.id,
                                &mut item.state,
                                element.downcast_mut().unwrap(),
                                app_state,
                            );
                            item.view = view;

                            if item_changed {
                                element.request_update();
                                changed = true;
                            }

                            // Our position changed
                            if old_index != children.len() {
                                changed = true;
                            }

                            children.push(element);
                            item
                        }
                        None => {
                            // We're adding new items
                            changed = true;
                            let (id, state, element) = view.build(cx, app_state);
                            children.push(Pod::new(element));
                            ItemState {
                                key,
                                id,
                                state,
                                view,
                            }
                        }
                    }
                })
                .collect();
            if !state.items.is_empty() {
                // We're removing existing items
                changed = true;
            }
            *element.children_mut() = children;
            *state = ForEachState { items };
        });

        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        let id = id_path[0];
        if let Some(item) = state.items.iter_mut().find(|item| item.id == id) {
            item.view
                .event(&id_path[1..], &mut item.state, event, app_state)
        } else {
            EventResult::Stale
        }
    }
}
