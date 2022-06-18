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

use crate::{event::EventResult, id::Id, View};

use super::Cx;

pub struct ScrollView<T, A, C> {
    child: C,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub fn scroll_view<T, A, C>(child: C) -> ScrollView<T, A, C> {
    ScrollView::new(child)
}

impl<T, A, C> ScrollView<T, A, C> {
    pub fn new(child: C) -> Self {
        ScrollView {
            child,
            phantom: Default::default(),
        }
    }
}

impl<T, A, C: View<T, A>> View<T, A> for ScrollView<T, A, C>
where
    C::Element: 'static,
{
    type State = C::State;

    type Element = crate::widget::scroll_view::ScrollView;

    fn build(&self, cx: &mut Cx) -> (Self::State, Self::Element) {
        let (child_state, child_element) = self.child.build(cx);
        let element = crate::widget::scroll_view::ScrollView::new(child_element);
        (child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        let child_element = element.child_mut().downcast_mut().unwrap();
        let changed = self.child.rebuild(cx, &prev.child, state, child_element);
        if changed {
            element.child_mut().request_update();
        }
        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        self.child.event(id_path, state, event, app_state)
    }
}
