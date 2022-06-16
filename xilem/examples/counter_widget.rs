use druid_shell::kurbo::Size;
use std::any::Any;
use std::fmt::Debug;
use xilem::widget::align::SingleAlignment;
use xilem::widget::AlignCx;
use xilem::{button, v_stack, App, AppLauncher, View};
use xilem::{
    widget::{
        self, text::TextWidget, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, RawEvent,
        UpdateCx,
    },
    Cx, EventResult, Id, Widget,
};

pub fn both_as_widget(cx: &mut Cx) -> impl Widget {
    let reactive_counter = Pod::new(ViewAsWidget::new(
        cx,
        ReactiveCounter::default(),
        reactive_counter,
    ));
    let sep = Pod::new(TextWidget::new("--------------------------".to_string()));
    let counter = Pod::new(CounterWidget::new(cx));

    println!(
        "reactive_counter root:{:?} id:{:?}",
        cx.id_path(),
        reactive_counter.id(),
    );

    println!("counter id:{:?}", counter.id());
    let stack = widget::vstack::VStack::new(vec![reactive_counter, sep, counter]);
    stack
}

fn app(data: &mut ()) -> impl View<()> {
    WidgetAsView(both_as_widget)
    /*
    v_stack((
        "Counter demo".to_string(),
        "--------------------------".to_string(),
        WidgetAsView(both_as_widget),
    )) */
}

pub fn main() {
    let app = App::new((), app);
    AppLauncher::new(app).run();
}

// Traditional GUI style counter
struct CounterWidget {
    id: Id,
    count: u32,
    stack: widget::vstack::VStack,
}

impl CounterWidget {
    pub fn new(cx: &mut Cx) -> Self {
        let (id, widgets) = cx.with_new_id(|cx| {
            vec![
                Pod::new(TextWidget::new(format!("Count: {}", 0))),
                Pod::new(widget::button::Button::new(
                    cx.id_path(),
                    "Increase".to_string(),
                )),
            ]
        });
        CounterWidget {
            id,
            count: 0,
            stack: widget::vstack::VStack::new(widgets),
        }
    }
}

impl Widget for CounterWidget {
    fn id(&self) -> Option<Id> {
        Some(self.id)
    }

    fn message(&mut self, cx: &mut Cx, _id_path: &[Id], _event: Box<dyn Any>) -> EventResult<()> {
        println!("got msg");
        self.count += 1;

        let label = &mut self.stack.children_mut()[0];
        label.request_update();
        let label: &mut TextWidget = label.downcast_mut().unwrap();
        label.set_text(format!("Count: {}", self.count));
        EventResult::RequestRebuild
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.stack.event(cx, event);
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.stack.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.stack.update(cx);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.stack.prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.stack.layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.stack.paint(cx);
    }
}

// Reactive counter
#[derive(Default, Debug)]
struct ReactiveCounter {
    count: u32,
}

fn reactive_counter(data: &mut ReactiveCounter) -> impl View<ReactiveCounter> + Debug {
    v_stack((
        format!("Reactive Count: {}", data.count),
        button("Increase".to_string(), |data: &mut ReactiveCounter| {
            println!("reactive_counter clicked");
            data.count += 1
        }),
    ))
}

// Use a view as a widget
struct ViewAsWidget<S, V: View<S>> {
    logic_state: S,
    view_tree_builder: fn(&mut S) -> V,
    view_state: V::State,
    view: V,
    element: Pod,
    id: Id,
    widget_id: Id,
}

impl<S, V: View<S>> ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
{
    fn new(cx: &mut Cx, mut logic_state: S, view_tree_builder: fn(&mut S) -> V) -> Self {
        let view = view_tree_builder(&mut logic_state);
        let (widget_id, (id, view_state, element)) = cx.with_new_id(|cx| view.build(cx));
        println!("ViewAsWidget widget_id:{:?}, id:{:?}", widget_id, id);
        let element = Pod::new(element);
        ViewAsWidget {
            view_tree_builder,
            view,
            view_state,
            logic_state,
            id,
            widget_id,
            element,
        }
    }
}

impl<S, V: View<S>> Widget for ViewAsWidget<S, V>
where
    V::Element: Widget + 'static,
    S: Debug,
    V: Debug,
{
    fn id(&self) -> Option<Id> {
        Some(self.widget_id)
    }

    fn message(&mut self, cx: &mut Cx, id_path: &[Id], event: Box<dyn Any>) -> EventResult<()> {
        println!(
            "message ViewAsWidget root:{:?}, path:{:?} state:{:?} view:{:?}",
            cx.id_path(),
            id_path,
            self.logic_state,
            self.view
        );
        let result = self.view.event(
            cx,
            &id_path[1..],
            &mut self.view_state,
            self.element.downcast_mut().unwrap(),
            event,
            &mut self.logic_state,
        );
        let view = (self.view_tree_builder)(&mut self.logic_state);
        let changed = self.view.rebuild(
            cx,
            &self.view,
            &mut self.id,
            &mut self.view_state,
            self.element.downcast_mut().unwrap(),
        );
        self.view = view;
        println!(
            "message ViewAsWidget root:{:?}, path:{:?} state:{:?} result:{:?} changed:{} view:{:?}",
            cx.id_path(),
            id_path,
            self.logic_state,
            result,
            changed,
            self.view
        );
        if changed {
            self.element.request_update();
        }
        result
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.element.update(cx);
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.element.event(cx, event);
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.element.prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.element.layout(cx, proposed_size)
    }

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        self.element.align(cx, alignment);
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }
}

// Use a widget as a view
struct WidgetAsView<T>(fn(&mut Cx) -> T);

impl<T: Widget> View<(), ()> for WidgetAsView<T> {
    type State = bool;

    type Element = T;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| self.0(cx));
        (id, false, element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        state: &mut Self::State,
        _element: &mut Self::Element,
    ) -> bool {
        std::mem::take(state)
    }

    fn event(
        &self,
        cx: &mut Cx,
        id_path: &[Id],
        state: &mut Self::State,
        element: &mut Self::Element,
        event: Box<dyn Any>,
        _app_state: &mut (),
    ) -> EventResult<()> {
        println!(
            "event WidgetAsView root:{:?}, path:{:?} ",
            cx.id_path(),
            id_path
        );
        //let id_path = &id_path[1..];
        let result = element.message(cx, id_path, event);
        if let EventResult::RequestRebuild = result {
            *state = true;
        }
        result
    }
}
