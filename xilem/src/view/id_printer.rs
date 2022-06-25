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

use crate::{event::EventResult, id::Id, widget::text::TextWidget};

use super::{Cx, View};

pub struct IdPrinter;

impl<T, A> View<T, A> for IdPrinter {
    type State = ();

    type Element = TextWidget;

    fn build(&self, cx: &mut Cx, _app_state: &mut T) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| {
            let segments: Vec<_> = cx
                .id_path()
                .iter()
                .map(|i| i.to_raw().to_string())
                .collect();
            TextWidget::new(segments.join(":"))
        });
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        _element: &mut Self::Element,
        _app_state: &mut T,
    ) -> bool {
        false
    }

    fn event(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        _app_state: &mut T,
    ) -> EventResult<A> {
        EventResult::Stale
    }
}
