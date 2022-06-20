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

use xilem::{
    button, v_stack, widget::Pod, Adapt, App, AppLauncher, Cx, EventResult, Id, LayoutObserver,
    Memoize, UseStateMut, View, ViewState,
};

#[derive(Default)]
struct CounterData {
    count: u32,
}

fn count_button(count: u32) -> impl View<u32> {
    button(format!("count: {}", count), |data| *data += 1)
}

fn counter(data: &mut CounterData) -> impl View<CounterData> {
    v_stack((
        format!("Child counter: {}", data.count),
        button("reset", |data: &mut CounterData| data.count = 0),
        Memoize::new(data.count, |count| {
            button(format!("count: {}", count), |data: &mut CounterData| {
                data.count += 1
            })
        }),
        Adapt::new(
            |data: &mut CounterData, thunk| thunk.call(&mut data.count),
            count_button(data.count),
        ),
        LayoutObserver::new(|size| format!("size: {:?}", size)),
    ))
}

pub struct AdaptChild<C> {
    child: C,
}

impl<C: ViewState> ViewState for AdaptChild<C> {
    type State = C::State;

    type Element = C::Element;
}

impl<'a, Outer, Inner, C: View<Inner>> View<(&'a mut Outer, &'a mut Inner)> for AdaptChild<C> {
    fn build(
        &self,
        cx: &mut Cx,
        app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> (Id, Self::State, Self::Element) {
        self.child.build(cx, app_state.1)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> bool {
        self.child
            .rebuild(cx, &prev.child, id, state, element, app_state.1)
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> EventResult<()> {
        self.child.event(id_path, state, event, app_state.1)
    }
}

pub struct Button<Outer, Inner> {
    label: String,
    callback: Box<dyn Fn(&mut (&mut Outer, &mut Inner)) -> () + Send>,
}

impl<Outer, Inner> ViewState for Button<Outer, Inner> {
    type State = ();
    type Element = xilem::widget::button::Button;
}

impl<'a, Outer, Inner> View<(&'a mut Outer, &'a mut Inner)> for Button<Outer, Inner> {
    fn build(
        &self,
        cx: &mut Cx,
        _app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx
            .with_new_id(|cx| xilem::widget::button::Button::new(cx.id_path(), self.label.clone()));
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
        _app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> bool {
        if prev.label != self.label {
            element.set_label(self.label.clone());
            true
        } else {
            false
        }
    }

    fn event(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        app_state: &mut (&'a mut Outer, &'a mut Inner),
    ) -> EventResult<()> {
        EventResult::Action((self.callback)(app_state))
    }
}

pub struct VSplit<T, B> {
    top: T,
    bottom: B,
}

impl<T: ViewState, B: ViewState> ViewState for VSplit<T, B> {
    type State = (Id, T::State, Id, B::State);

    type Element = xilem::widget::vstack::VStack;
}

impl<G, T: View<G>, B: View<G>> View<G> for VSplit<T, B>
where
    <T as ViewState>::Element: 'static,
    <B as ViewState>::Element: 'static,
{
    fn build(&self, cx: &mut Cx, app_state: &mut G) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (id1, state1, element1) = self.top.build(cx, app_state);
            let (id2, state2, element2) = self.bottom.build(cx, app_state);
            (
                (id1, state1, id2, state2),
                xilem::widget::vstack::VStack::new(vec![Pod::new(element1), Pod::new(element2)]),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
        app_state: &mut G,
    ) -> bool {
        let mut changed = false;
        cx.with_id(*id, |cx| {
            if self.top.rebuild(
                cx,
                &prev.top,
                &mut state.0,
                &mut state.1,
                element.children_mut()[0].downcast_mut().unwrap(),
                app_state,
            ) {
                element.children_mut()[0].request_update();
                changed = true;
            }
            if self.bottom.rebuild(
                cx,
                &prev.bottom,
                &mut state.2,
                &mut state.3,
                element.children_mut()[1].downcast_mut().unwrap(),
                app_state,
            ) {
                element.children_mut()[1].request_update();
                changed = true;
            }
        });
        true
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut G,
    ) -> EventResult<()> {
        if id_path[0] == state.0 {
            self.top
                .event(&id_path[1..], &mut state.1, event, app_state)
        } else {
            self.bottom
                .event(&id_path[1..], &mut state.3, event, app_state)
        }
    }
}

fn adapter(
    parent: &mut ParentCounterData,
    data: &mut CounterData,
) -> impl for<'a> View<(&'a mut ParentCounterData, &'a mut CounterData)> {
    VSplit {
        top: Button {
            label: format!("Increase parent count: {}", parent.count),
            callback: Box::new(|data: &mut (&mut ParentCounterData, &mut CounterData)| {
                data.0.count += 1;
            }),
        },
        bottom: AdaptChild {
            child: counter(data),
        },
    }
}

#[derive(Default)]
struct ParentCounterData {
    count: u32,
}

fn app_logic(data: &mut ParentCounterData) -> impl View<ParentCounterData> {
    v_stack((
        format!("Parent counter: {}", data.count),
        UseStateMut::new(|| CounterData::default(), adapter),
    ))
}

pub fn main() {
    let app = App::new(ParentCounterData::default(), app_logic);
    AppLauncher::new(app).run();
}
