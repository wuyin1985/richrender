use rich_engine::{ElementState, InputSystem, MouseScrollUnit, MouseWheel, RenderInitEvent, RenderRunner, SystemParam};
use rich_engine::prelude::*;

pub use egui;


#[cfg(all(feature = "manage_clipboard", not(target_arch = "wasm32")))]
use clipboard::{ClipboardContext, ClipboardProvider};
#[cfg(all(feature = "manage_clipboard", not(target_arch = "wasm32")))]
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::ops::DerefMut;
#[cfg(all(feature = "manage_clipboard", not(target_arch = "wasm32")))]
use thread_local::ThreadLocal;
use rich_engine::window::{WindowCreated, WindowFocused};
use rich_engine::bevy_winit::WinitWindows;
use rich_engine::keyboard::KeyboardInput;
use crate::egui_render::EguiRender;


/// Adds all Egui resources and render graph nodes.
pub struct EguiPlugin;

/// A resource for storing global UI settings.
#[derive(Clone, Debug, PartialEq)]
pub struct EguiSettings {
    pub scale_factor: f64,
}

impl Default for EguiSettings {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

/// Is used for storing the input passed to Egui. The actual resource is a [`HashMap<WindowId, EguiInput>`].
///
/// It gets reset during the [`EguiSystem::ProcessInput`] system.
#[derive(Clone, Debug, Default)]
pub struct EguiInput {
    /// Egui's raw input.
    pub raw_input: egui::RawInput,
}

/// A resource for accessing clipboard.
///
/// The resource is available only if `manage_clipboard` feature is enabled.
#[cfg(feature = "manage_clipboard")]
#[derive(Default)]
pub struct EguiClipboard {
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: ThreadLocal<Option<RefCell<ClipboardContext>>>,
    #[cfg(target_arch = "wasm32")]
    clipboard: String,
}

#[cfg(feature = "manage_clipboard")]
impl EguiClipboard {
    /// Sets clipboard contents.
    pub fn set_contents(&mut self, contents: &str) {
        self.set_contents_impl(contents);
    }

    /// Gets clipboard contents. Returns [`None`] if clipboard provider is unavailable or returns an error.
    pub fn get_contents(&self) -> Option<String> {
        self.get_contents_impl()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_contents_impl(&self, contents: &str) {
        if let Some(mut clipboard) = self.get() {
            if let Err(err) = clipboard.set_contents(contents.to_owned()) {
                error!("Failed to set clipboard contents: {:?}", err);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn set_contents_impl(&mut self, contents: &str) {
        self.clipboard = contents.to_owned();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_contents_impl(&self) -> Option<String> {
        if let Some(mut clipboard) = self.get() {
            match clipboard.get_contents() {
                Ok(contents) => return Some(contents),
                Err(err) => info!("Failed to get clipboard contents: {:?}", err),
            }
        };
        None
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unnecessary_wraps)]
    fn get_contents_impl(&self) -> Option<String> {
        Some(self.clipboard.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get(&self) -> Option<RefMut<ClipboardContext>> {
        self.clipboard
            .get_or(|| {
                ClipboardContext::new()
                    .map(RefCell::new)
                    .map_err(|err| {
                        info!("Failed to initialize clipboard: {:?}", err);
                    })
                    .ok()
            })
            .as_ref()
            .map(|cell| cell.borrow_mut())
    }
}

/// Is used for storing Egui output. The actual resource is [`HashMap<WindowId, EguiOutput>`].
#[derive(Clone, Default)]
pub struct EguiOutput {
    /// The field gets updated during the [`EguiStage::UiFrameEnd`] stage.
    pub output: egui::Output,
}

/// A resource for storing `bevy_egui` context.
pub struct EguiContext {
    ctx: HashMap<WindowId, egui::CtxRef>,
    mouse_position: Option<(f32, f32)>,
    pub render: Option<EguiRender>,
}

impl EguiContext {
    fn new() -> Self {
        Self {
            ctx: HashMap::default(),
            mouse_position: Some((0.0, 0.0)),
            render: None,
        }
    }

    #[track_caller]
    pub fn ctx(&self) -> &egui::CtxRef {
        self.ctx.get(&WindowId::primary()).expect("`EguiContext::ctx()` called before the ctx has been initialized. Consider moving your UI system to `CoreStage::Update` or run you system after `EguiSystem::BeginFrame`.")
    }

    /// Egui context for a specific window.
    /// If you want to display UI on a non-primary window,
    /// make sure to set up the render graph by calling [`setup_pipeline`].
    ///
    /// Note: accessing the context from different threads simultaneously requires enabling
    /// `egui/multi_threaded` feature.
    #[track_caller]
    pub fn ctx_for_window(&self, window: WindowId) -> &egui::CtxRef {
        self.ctx
            .get(&window)
            .ok_or_else(|| format!("window with id {} not found", window))
            .unwrap()
    }

    /// Fallible variant of [`EguiContext::ctx_for_window`]. Make sure to set up the render graph by calling [`setup_pipeline`].
    pub fn try_ctx_for_window(&self, window: WindowId) -> Option<&egui::CtxRef> {
        self.ctx.get(&window)
    }
}

#[doc(hidden)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct WindowSize {
    physical_width: f32,
    physical_height: f32,
    scale_factor: f32,
}

impl WindowSize {
    fn new(physical_width: f32, physical_height: f32, scale_factor: f32) -> Self {
        Self {
            physical_width,
            physical_height,
            scale_factor,
        }
    }

    #[inline]
    fn width(&self) -> f32 {
        self.physical_width / self.scale_factor
    }

    #[inline]
    fn height(&self) -> f32 {
        self.physical_height / self.scale_factor
    }
}


#[derive(SystemLabel, Clone, Hash, Debug, Eq, PartialEq)]
/// The names of egui systems.
pub enum EguiSystem {
    ProcessInput,
    /// Begins the `egui` frame
    BeginFrame,
    /// Processes the [`EguiOutput`] resource
    ProcessOutput,
}

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(
            CoreStage::PreUpdate,
            process_input.system()
                .label(EguiSystem::ProcessInput)
                .after(InputSystem),
        );
        app.add_system_to_stage(
            CoreStage::PreUpdate,
            begin_frame
                .system()
                .label(EguiSystem::BeginFrame)
                .after(EguiSystem::ProcessInput),
        );
        app.add_system_to_stage(
            RenderStage::PrepareDraw,
            init_egui_ctx.system(),
        );
        app.add_system_to_stage(
            RenderStage::Upload,
            upload_egui_data_2_render.system(),
        );
        app.add_system_to_stage(
            RenderStage::Draw,
            process_output.system().label(EguiSystem::ProcessOutput),
        );

        let world = app.world_mut();
        world.get_resource_or_insert_with(EguiSettings::default);
        world.get_resource_or_insert_with(HashMap::<WindowId, EguiInput>::default);
        world.get_resource_or_insert_with(HashMap::<WindowId, EguiOutput>::default);
        world.get_resource_or_insert_with(HashMap::<WindowId, WindowSize>::default);
        #[cfg(feature = "manage_clipboard")]
            world.get_resource_or_insert_with(EguiClipboard::default);
        world.insert_resource(EguiContext::new());
    }
}


#[derive(SystemParam)]
pub struct InputEvents<'a> {
    ev_cursor_left: EventReader<'a, CursorLeft>,
    ev_cursor: EventReader<'a, CursorMoved>,
    ev_mouse_wheel: EventReader<'a, MouseWheel>,
    ev_received_character: EventReader<'a, ReceivedCharacter>,
    ev_keyboard_input: EventReader<'a, KeyboardInput>,
    ev_window_focused: EventReader<'a, WindowFocused>,
    ev_window_created: EventReader<'a, WindowCreated>,
}

#[derive(SystemParam)]
pub struct InputResources<'a> {
    #[cfg(feature = "manage_clipboard")]
    egui_clipboard: Res<'a, EguiClipboard>,
    mouse_button_input: Res<'a, Input<MouseButton>>,
    keyboard_input: Res<'a, Input<KeyCode>>,
    egui_input: ResMut<'a, HashMap<WindowId, EguiInput>>,
}

#[derive(SystemParam)]
pub struct WindowResources<'a> {
    focused_window: Local<'a, WindowId>,
    windows: ResMut<'a, Windows>,
    window_sizes: ResMut<'a, HashMap<WindowId, WindowSize>>,
}

fn init_egui_ctx(mut ctx: ResMut<EguiContext>, mut runner: Option<ResMut<RenderRunner>>, mut init_events: EventReader<RenderInitEvent>) {
    if ctx.render.is_some() {
        return;
    }
    if let Some(runner) = &mut runner {
        let runner = runner.deref_mut();
        let context = &mut runner.context;
        let render = EguiRender::new(context.window_width, context.window_height, 1.0,
                                     ctx.ctx().clone(), context.render_config.color_format, context, &runner.forward_render_pass);
        
        ctx.render = Some(render);
    }
}


pub fn process_input(
    mut egui_context: ResMut<EguiContext>,
    mut input_events: InputEvents,
    mut input_resources: InputResources,
    mut window_resources: WindowResources,
    egui_settings: ResMut<EguiSettings>,
    time: Res<Time>,
) {
    // This is a workaround for Windows. For some reason, `WindowFocused` event isn't fired.
    // when a window is created.
    for event in input_events.ev_window_created.iter().rev() {
        *window_resources.focused_window = event.id;
    }

    for event in input_events.ev_window_focused.iter().rev() {
        if event.focused {
            *window_resources.focused_window = event.id;
        }
    }

    for window in window_resources.windows.iter() {
        let egui_input = input_resources.egui_input.entry(window.id()).or_default();

        let window_size = WindowSize::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
            window.scale_factor() as f32,
        );
        let width = window_size.physical_width
            / window_size.scale_factor
            / egui_settings.scale_factor as f32;
        let height = window_size.physical_height
            / window_size.scale_factor
            / egui_settings.scale_factor as f32;

        if width < 1.0 || height < 1.0 {
            continue;
        }

        egui_input.raw_input.screen_rect = Some(egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(width, height),
        ));

        egui_input.raw_input.pixels_per_point =
            Some(window_size.scale_factor * egui_settings.scale_factor as f32);

        window_resources
            .window_sizes
            .insert(window.id(), window_size);
        egui_context.ctx.entry(window.id()).or_default();
    }

    for event in input_events.ev_mouse_wheel.iter() {
        let mut delta = egui::vec2(event.x, event.y);
        if let MouseScrollUnit::Line = event.unit {
            // TODO: https://github.com/emilk/egui/blob/b869db728b6bbefa098ac987a796b2b0b836c7cd/egui_glium/src/lib.rs#L141
            delta *= 24.0;
        }

        for egui_input in input_resources.egui_input.values_mut() {
            egui_input.raw_input.scroll_delta += delta;
        }
    }

    let shift = input_resources.keyboard_input.pressed(KeyCode::LShift)
        || input_resources.keyboard_input.pressed(KeyCode::RShift);
    let ctrl = input_resources.keyboard_input.pressed(KeyCode::LControl)
        || input_resources.keyboard_input.pressed(KeyCode::RControl);
    let alt = input_resources.keyboard_input.pressed(KeyCode::LAlt)
        || input_resources.keyboard_input.pressed(KeyCode::RAlt);
    let win = input_resources.keyboard_input.pressed(KeyCode::LWin)
        || input_resources.keyboard_input.pressed(KeyCode::RWin);

    let mac_cmd = if cfg!(target_os = "macos") {
        win
    } else {
        false
    };
    let command = if cfg!(target_os = "macos") { win } else { ctrl };

    let modifiers = egui::Modifiers {
        alt,
        ctrl,
        shift,
        mac_cmd,
        command,
    };

    for cursor_entered in input_events.ev_cursor_left.iter() {
        input_resources
            .egui_input
            .get_mut(&cursor_entered.id)
            .unwrap()
            .raw_input
            .events
            .push(egui::Event::PointerGone);
        egui_context.mouse_position = None;
    }
    if let Some(cursor_moved) = input_events.ev_cursor.iter().next_back() {
        let scale_factor = egui_settings.scale_factor as f32;
        let mut mouse_position: (f32, f32) = (cursor_moved.position / scale_factor).into();
        mouse_position.1 = window_resources.window_sizes[&cursor_moved.id].height() / scale_factor
            - mouse_position.1;
        egui_context.mouse_position = Some(mouse_position);
        input_resources
            .egui_input
            .get_mut(&cursor_moved.id)
            .unwrap()
            .raw_input
            .events
            .push(egui::Event::PointerMoved(egui::pos2(
                mouse_position.0,
                mouse_position.1,
            )));
    }

    if let Some((x, y)) = egui_context.mouse_position {
        let focused_egui_input = input_resources
            .egui_input
            .get_mut(&*window_resources.focused_window)
            .unwrap();
        let events = &mut focused_egui_input.raw_input.events;

        let pos = egui::pos2(x, y);
        process_mouse_button_event(
            events,
            pos,
            modifiers,
            &input_resources.mouse_button_input,
            MouseButton::Left,
        );
        process_mouse_button_event(
            events,
            pos,
            modifiers,
            &input_resources.mouse_button_input,
            MouseButton::Right,
        );
        process_mouse_button_event(
            events,
            pos,
            modifiers,
            &input_resources.mouse_button_input,
            MouseButton::Middle,
        );
    }

    if !ctrl && !win {
        for event in input_events.ev_received_character.iter() {
            if !event.char.is_control() {
                input_resources
                    .egui_input
                    .get_mut(&event.id)
                    .unwrap()
                    .raw_input
                    .events
                    .push(egui::Event::Text(event.char.to_string()));
            }
        }
    }

    let focused_input = input_resources
        .egui_input
        .get_mut(&*window_resources.focused_window)
        .unwrap();

    for ev in input_events.ev_keyboard_input.iter() {
        if let Some(key) = ev.key_code.and_then(bevy_to_egui_key) {
            let egui_event = egui::Event::Key {
                key,
                pressed: match ev.state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                },
                modifiers,
            };
            focused_input.raw_input.events.push(egui_event);

            #[cfg(feature = "manage_clipboard")]
            if command {
                match key {
                    egui::Key::C => {
                        focused_input.raw_input.events.push(egui::Event::Copy);
                    }
                    egui::Key::X => {
                        focused_input.raw_input.events.push(egui::Event::Cut);
                    }
                    egui::Key::V => {
                        if let Some(contents) = input_resources.egui_clipboard.get_contents() {
                            focused_input
                                .raw_input
                                .events
                                .push(egui::Event::Text(contents))
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    focused_input.raw_input.modifiers = modifiers;

    for egui_input in input_resources.egui_input.values_mut() {
        egui_input.raw_input.predicted_dt = time.delta_seconds();
    }
}

pub fn begin_frame(
    mut egui_context: ResMut<EguiContext>,
    mut egui_input: ResMut<HashMap<WindowId, EguiInput>>,
) {
    let ids: Vec<_> = egui_context.ctx.keys().copied().collect();
    for id in ids {
        let raw_input = egui_input.get_mut(&id).unwrap().raw_input.take();
        egui_context
            .ctx
            .get_mut(&id)
            .unwrap()
            .begin_frame(raw_input);
    }
}

pub fn process_output(
    mut egui_context: ResMut<EguiContext>,
    mut runner: Option<ResMut<RenderRunner>>,
    #[cfg(feature = "manage_clipboard")] mut egui_clipboard: ResMut<EguiClipboard>,
    winit_windows: Res<WinitWindows>,
) {
    if let Some(runner) = &mut runner {
        let egui_context = egui_context.deref_mut();
        for id in egui_context.ctx.keys().copied() {
            let (output, shapes) = egui_context.ctx_for_window(id).end_frame();

            #[cfg(feature = "manage_clipboard")]
            if !output.copied_text.is_empty() {
                egui_clipboard.set_contents(&output.copied_text);
            }

            if let Some(winit_window) = winit_windows.get_window(id) {
                winit_window.set_cursor_icon(
                    egui_to_winit_cursor_icon(output.cursor_icon)
                        .unwrap_or(rich_engine::CursorIcon::Default),
                );
            }

            let clipped_meshes = egui_context.ctx().tessellate(shapes);
            if let Some(rt) = &mut egui_context.render {
                let cd = runner.get_current_command_buffer().unwrap();
                let context = &mut runner.context;
                rt.paint(context, cd, clipped_meshes);
            }

            // TODO: see if we can support `new_tab`.
            #[cfg(feature = "open_url")]
            if let Some(egui::output::OpenUrl {
                            url,
                            new_tab: _new_tab,
                        }) = output.open_url
            {
                if let Err(err) = webbrowser::open(&url) {
                    error!("Failed to open '{}': {:?}", url, err);
                }
            }
        }
    }
}

fn egui_to_winit_cursor_icon(cursor_icon: egui::CursorIcon) -> Option<rich_engine::CursorIcon> {
    match cursor_icon {
        egui::CursorIcon::Default => Some(rich_engine::CursorIcon::Default),
        egui::CursorIcon::PointingHand => Some(rich_engine::CursorIcon::Hand),
        egui::CursorIcon::ResizeHorizontal => Some(rich_engine::CursorIcon::EwResize),
        egui::CursorIcon::ResizeNeSw => Some(rich_engine::CursorIcon::NeswResize),
        egui::CursorIcon::ResizeNwSe => Some(rich_engine::CursorIcon::NwseResize),
        egui::CursorIcon::ResizeVertical => Some(rich_engine::CursorIcon::NsResize),
        egui::CursorIcon::Text => Some(rich_engine::CursorIcon::Text),
        egui::CursorIcon::Grab => Some(rich_engine::CursorIcon::Grab),
        egui::CursorIcon::Grabbing => Some(rich_engine::CursorIcon::Grabbing),
        egui::CursorIcon::ContextMenu => Some(rich_engine::CursorIcon::ContextMenu),
        egui::CursorIcon::Help => Some(rich_engine::CursorIcon::Help),
        egui::CursorIcon::Progress => Some(rich_engine::CursorIcon::Progress),
        egui::CursorIcon::Wait => Some(rich_engine::CursorIcon::Wait),
        egui::CursorIcon::Cell => Some(rich_engine::CursorIcon::Cell),
        egui::CursorIcon::Crosshair => Some(rich_engine::CursorIcon::Crosshair),
        egui::CursorIcon::VerticalText => Some(rich_engine::CursorIcon::VerticalText),
        egui::CursorIcon::Alias => Some(rich_engine::CursorIcon::Alias),
        egui::CursorIcon::Copy => Some(rich_engine::CursorIcon::Copy),
        egui::CursorIcon::Move => Some(rich_engine::CursorIcon::Move),
        egui::CursorIcon::NoDrop => Some(rich_engine::CursorIcon::NoDrop),
        egui::CursorIcon::NotAllowed => Some(rich_engine::CursorIcon::NotAllowed),
        egui::CursorIcon::AllScroll => Some(rich_engine::CursorIcon::AllScroll),
        egui::CursorIcon::ZoomIn => Some(rich_engine::CursorIcon::ZoomIn),
        egui::CursorIcon::ZoomOut => Some(rich_engine::CursorIcon::ZoomOut),
        egui::CursorIcon::None => None,
    }
}

fn bevy_to_egui_key(key_code: KeyCode) -> Option<egui::Key> {
    let key = match key_code {
        KeyCode::Down => egui::Key::ArrowDown,
        KeyCode::Left => egui::Key::ArrowLeft,
        KeyCode::Right => egui::Key::ArrowRight,
        KeyCode::Up => egui::Key::ArrowUp,
        KeyCode::Escape => egui::Key::Escape,
        KeyCode::Tab => egui::Key::Tab,
        KeyCode::Back => egui::Key::Backspace,
        KeyCode::Return => egui::Key::Enter,
        KeyCode::Space => egui::Key::Space,
        KeyCode::Insert => egui::Key::Insert,
        KeyCode::Delete => egui::Key::Delete,
        KeyCode::Home => egui::Key::Home,
        KeyCode::End => egui::Key::End,
        KeyCode::PageUp => egui::Key::PageUp,
        KeyCode::PageDown => egui::Key::PageDown,
        KeyCode::Numpad0 | KeyCode::Key0 => egui::Key::Num0,
        KeyCode::Numpad1 | KeyCode::Key1 => egui::Key::Num1,
        KeyCode::Numpad2 | KeyCode::Key2 => egui::Key::Num2,
        KeyCode::Numpad3 | KeyCode::Key3 => egui::Key::Num3,
        KeyCode::Numpad4 | KeyCode::Key4 => egui::Key::Num4,
        KeyCode::Numpad5 | KeyCode::Key5 => egui::Key::Num5,
        KeyCode::Numpad6 | KeyCode::Key6 => egui::Key::Num6,
        KeyCode::Numpad7 | KeyCode::Key7 => egui::Key::Num7,
        KeyCode::Numpad8 | KeyCode::Key8 => egui::Key::Num8,
        KeyCode::Numpad9 | KeyCode::Key9 => egui::Key::Num9,
        KeyCode::A => egui::Key::A,
        KeyCode::B => egui::Key::B,
        KeyCode::C => egui::Key::C,
        KeyCode::D => egui::Key::D,
        KeyCode::E => egui::Key::E,
        KeyCode::F => egui::Key::F,
        KeyCode::G => egui::Key::G,
        KeyCode::H => egui::Key::H,
        KeyCode::I => egui::Key::I,
        KeyCode::J => egui::Key::J,
        KeyCode::K => egui::Key::K,
        KeyCode::L => egui::Key::L,
        KeyCode::M => egui::Key::M,
        KeyCode::N => egui::Key::N,
        KeyCode::O => egui::Key::O,
        KeyCode::P => egui::Key::P,
        KeyCode::Q => egui::Key::Q,
        KeyCode::R => egui::Key::R,
        KeyCode::S => egui::Key::S,
        KeyCode::T => egui::Key::T,
        KeyCode::U => egui::Key::U,
        KeyCode::V => egui::Key::V,
        KeyCode::W => egui::Key::W,
        KeyCode::X => egui::Key::X,
        KeyCode::Y => egui::Key::Y,
        KeyCode::Z => egui::Key::Z,
        _ => return None,
    };
    Some(key)
}

fn process_mouse_button_event(
    egui_events: &mut Vec<egui::Event>,
    pos: egui::Pos2,
    modifiers: egui::Modifiers,
    mouse_button_input: &Input<MouseButton>,
    mouse_button: MouseButton,
) {
    let button = match mouse_button {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        _ => panic!("Unsupported mouse button"),
    };

    let pressed = if mouse_button_input.just_pressed(mouse_button) {
        true
    } else if mouse_button_input.just_released(mouse_button) {
        false
    } else {
        return;
    };
    egui_events.push(egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    });
}

fn upload_egui_data_2_render(mut egui_context: ResMut<EguiContext>, mut runner: Option<ResMut<RenderRunner>>) {
    if let Some(runner) = &mut runner {
        if let Some(rt) = &mut egui_context.render {
            let runner: &mut RenderRunner = runner.deref_mut();
            let command_buffer = runner.get_upload_command_buffer();
            rt.prepare(&mut runner.context, command_buffer);
        }
    }
}


