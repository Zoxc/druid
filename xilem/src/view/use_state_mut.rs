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

use std::{any::Any, marker::PhantomData};

use crate::{event::EventResult, id::Id};

use super::{Cx, View, ViewState};

/// An implementation of the "use_state" pattern familiar in reactive UI.
///
/// The first callback creates the initial state (it is called on build but
/// not rebuild). The second callback takes that state as an argument. It
/// is not passed the app state, but since that state is `Rc`, it would be
/// natural to clone it and capture it in a `move` closure.
pub struct UseStateMut<S, V, FInit, F> {
    f_init: FInit,
    f: F,
    phantom: PhantomData<fn() -> (S, V)>,
}

pub struct UseStateStateMut<S, V, VS> {
    local_state: S,
    view: V,
    view_state: VS,
}

impl<S, V, FInit, F> UseStateMut<S, V, FInit, F> {
    #[allow(unused)]
    pub fn new(f_init: FInit, f: F) -> Self {
        let phantom = Default::default();
        UseStateMut { f_init, f, phantom }
    }
}

impl<S, V, FInit, F> ViewState for UseStateMut<S, V, FInit, F>
where
    S: Send,
    FInit: Send,
    F: Send,
    V: ViewState + Send,
{
    type State = UseStateStateMut<S, V, V::State>;

    type Element = <V as ViewState>::Element;
}

impl<T, A, S, V, FInit: Fn() -> S, F: Fn(&mut T, &mut S) -> V> View<T, A>
    for UseStateMut<S, V, FInit, F>
where
    S: Send,
    FInit: Send,
    F: Send,
    V: ViewState + for<'a> View<(&'a mut T, &'a mut S), A>,
{
    fn build(&self, cx: &mut Cx, app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let mut local_state = (self.f_init)();
        let view = (self.f)(app_state, &mut local_state);
        let (id, view_state, element) = view.build(cx, &mut (app_state, &mut local_state));
        let state = UseStateStateMut {
            local_state,
            view,
            view_state,
        };
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
        let view = (self.f)(app_state, &mut state.local_state);
        let changed = view.rebuild(
            cx,
            &state.view,
            id,
            &mut state.view_state,
            element,
            &mut (app_state, &mut state.local_state),
        );
        state.view = view;
        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        state.view.event(
            id_path,
            &mut state.view_state,
            event,
            &mut (app_state, &mut state.local_state),
        )
    }
}
