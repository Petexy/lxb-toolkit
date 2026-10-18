#![allow(clippy::missing_safety_doc)]
use std::ffi::{c_char, c_float, c_int, c_uint, c_void, CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use lxb_app::lxb_render::{Align, Fit, Press};
use lxb_app::lxb_toolkit::{
    material::{Overlay, Surface},
    metrics::Metric,
    palette::Role,
    picker::Selection as PickerSelection,
    sound::Sound,
    typography::Text,
};
use lxb_app::{App, Page, Typed};

#[allow(non_camel_case_types)]
type Size = std::os::raw::c_ulong;

pub struct AppHandle {
    app: Option<App>,
    trouble: Option<CString>,
}

pub type PageFn = extern "C" fn(page: *mut Page<'static>, data: *mut c_void);

#[no_mangle]
pub unsafe extern "C" fn lxb_app_new(
    app_id: *const c_char,
    title: *const c_char,
) -> *mut AppHandle {
    let app_id = borrow(app_id).unwrap_or("com.example.Application");
    let title = borrow(title).unwrap_or("Application");
    Box::into_raw(Box::new(AppHandle {
        app: Some(App::new(app_id, title)),
        trouble: None,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_size(app: *mut AppHandle, width: f64, height: f64) {
    if let Some(handle) = app.as_mut() {
        handle.app = handle.app.take().map(|app| app.size(width, height));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_plain(app: *mut AppHandle) {
    if let Some(handle) = app.as_mut() {
        handle.app = handle.app.take().map(App::plain);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_driven(app: *mut AppHandle) {
    if let Some(handle) = app.as_mut() {
        handle.app = handle.app.take().map(App::driven);
    }
}

/// Count the session as low-end hardware whatever the shell's Settings say.
#[no_mangle]
pub unsafe extern "C" fn lxb_app_low_end(app: *mut AppHandle) {
    if let Some(handle) = app.as_mut() {
        handle.app = handle.app.take().map(App::low_end);
    }
}

/// A mark of the application's own, drawn by `name` like the toolkit's.
#[no_mangle]
pub unsafe extern "C" fn lxb_app_mark(
    app: *mut AppHandle,
    name: *const c_char,
    svg: *const c_void,
    length: Size,
) -> c_int {
    let Some(handle) = app.as_mut() else {
        return 0;
    };
    let refused = match (handle.app.as_mut(), borrow(name)) {
        (None, _) => "this application has already been run".to_string(),
        (_, None) => "a mark needs a name".to_string(),
        (Some(_), Some(_)) if svg.is_null() => "a mark needs a drawing".to_string(),
        (Some(built), Some(name)) => {
            let drawing = std::slice::from_raw_parts(svg.cast::<u8>(), length as usize);
            match built.add_mark(name, drawing) {
                Ok(()) => return 1,
                Err(message) => message,
            }
        }
    };
    handle.trouble = CString::new(refused).ok();
    0
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_run(
    app: *mut AppHandle,
    page: Option<PageFn>,
    data: *mut c_void,
) -> c_int {
    let Some(handle) = app.as_mut() else {
        return 1;
    };
    let Some(built) = handle.app.take() else {
        handle.trouble = CString::new("this application has already been run").ok();
        return 1;
    };
    let Some(page) = page else {
        handle.trouble = CString::new("an application with no page to draw").ok();
        return 1;
    };

    let outcome = built.run(|frame| {
        let handed: *mut Page<'_> = frame;
        page(handed.cast::<Page<'static>>(), data);
    });
    match outcome {
        Ok(()) => 0,
        Err(message) => {
            handle.trouble = CString::new(message).ok();
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_shot(
    app: *mut AppHandle,
    path: *const c_char,
    width: c_uint,
    height: c_uint,
    seconds: c_float,
    page: Option<PageFn>,
    data: *mut c_void,
) -> c_int {
    let Some(handle) = app.as_mut() else {
        return 1;
    };
    let Some(built) = handle.app.take() else {
        handle.trouble = CString::new("this application has already been run").ok();
        return 1;
    };
    let (Some(page), Some(path)) = (page, borrow(path)) else {
        handle.trouble = CString::new("a picture with no page to draw, or nowhere to put it").ok();
        return 1;
    };
    let outcome = built.shot(path, width, height, seconds, |frame| {
        let handed: *mut Page<'_> = frame;
        page(handed.cast::<Page<'static>>(), data);
    });
    match outcome {
        Ok(()) => 0,
        Err(message) => {
            handle.trouble = CString::new(message).ok();
            1
        }
    }
}

/// A picture of the window `frames` frames after its first, a sixtieth of a
/// second apart. Zero if it was written.
#[no_mangle]
pub unsafe extern "C" fn lxb_app_shot_after(
    app: *mut AppHandle,
    path: *const c_char,
    width: c_uint,
    height: c_uint,
    seconds: c_float,
    frames: c_uint,
    page: Option<PageFn>,
    data: *mut c_void,
) -> c_int {
    let Some(handle) = app.as_mut() else {
        return 1;
    };
    let Some(built) = handle.app.take() else {
        handle.trouble = CString::new("this application has already been run").ok();
        return 1;
    };
    let (Some(page), Some(path)) = (page, borrow(path)) else {
        handle.trouble = CString::new("a picture with no page to draw, or nowhere to put it").ok();
        return 1;
    };
    let outcome = built.shot_after(path, width, height, seconds, frames, |frame| {
        let handed: *mut Page<'_> = frame;
        page(handed.cast::<Page<'static>>(), data);
    });
    match outcome {
        Ok(()) => 0,
        Err(message) => {
            handle.trouble = CString::new(message).ok();
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_trouble(app: *const AppHandle) -> *const c_char {
    app.as_ref()
        .and_then(|handle| handle.trouble.as_ref())
        .map_or(std::ptr::null(), |trouble| trouble.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_free(app: *mut AppHandle) {
    if !app.is_null() {
        drop(Box::from_raw(app));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_width(page: *const Page<'static>) -> c_float {
    page.as_ref().map_or(0.0, |page| page.width())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_height(page: *const Page<'static>) -> c_float {
    page.as_ref().map_or(0.0, |page| page.height())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_seconds(page: *const Page<'static>) -> c_float {
    page.as_ref().map_or(0.0, |page| page.seconds())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_action(page: *mut Page<'static>) -> c_int {
    page.as_mut()
        .and_then(|page| page.actions().next())
        .map_or(-1, |action| action as c_int)
}

/// Hand the window an action, one of the `LXB_ACTION_` numbers, as a key or
/// a controller would. A number past the list does nothing — and so does
/// `lxb_page_action`'s -1 for nothing handed straight back, which arrives
/// here as the largest `unsigned long`.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_act(page: *mut Page<'static>, action: Size) {
    let Some(action) = lxb_app::lxb_toolkit::input::Action::ALL
        .get(action as usize)
        .copied()
    else {
        return;
    };
    if let Some(page) = page.as_mut() {
        page.act(action);
    }
}

/// How far a wheel or touchpad moved over one of this page's own spots,
/// counted in directions and signed downwards.
///
/// The directions themselves are already in `lxb_page_action`, so a page that
/// never calls this still scrolls. Call it to decide *which* of the page's
/// lists this frame's Up and Down move — the one question a pointer asks and
/// an action cannot answer.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_scrolled(page: *const Page<'static>, id: c_uint) -> c_int {
    page.as_ref().map_or(0, |page| page.scrolled(id) as c_int)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_focus(page: *mut Page<'static>, index: Size) {
    if let Some(page) = page.as_mut() {
        page.focus(index as usize);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_focused(page: *const Page<'static>) -> Size {
    page.as_ref().map_or(0, |page| page.focused() as Size)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_glide(
    page: *mut Page<'static>,
    rect: *const c_float,
    strength: c_float,
    out: *mut c_float,
) {
    travel(page, rect, strength, out, false)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_place(
    page: *mut Page<'static>,
    rect: *const c_float,
    strength: c_float,
    out: *mut c_float,
) {
    travel(page, rect, strength, out, true)
}

unsafe fn travel(
    page: *mut Page<'static>,
    rect: *const c_float,
    strength: c_float,
    out: *mut c_float,
    appears: bool,
) {
    let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) else {
        return;
    };
    let at = if appears {
        page.place(rect, strength)
    } else {
        page.glide(rect, strength)
    };
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(at.as_ptr(), out, 4);
    }
}

/// The page's light carried to `rect` — or put there at once, `appears` — and
/// drawn in the rectangle's own shape, its corners rounded by `radius`. Writes
/// where it is drawn this frame to `out`, or nothing without a page or a
/// rectangle.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_light_shaped(
    page: *mut Page<'static>,
    rect: *const c_float,
    radius: c_float,
    strength: c_float,
    appears: c_int,
    out: *mut c_float,
) {
    let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) else {
        return;
    };
    let at = page.light_shaped(rect, radius, strength, appears != 0);
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(at.as_ptr(), out, 4);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_press(page: *const Page<'static>, lit: c_int) -> Size {
    match page.as_ref() {
        Some(page) => match page.press(lit != 0) {
            Press::Resting => 0,
            Press::Focused => 1,
            Press::Pressed => 2,
            Press::Going(through) => 3 + (through.clamp(0.0, 1.0) * 100.0).round() as Size,
        },
        None => 0,
    }
}

/// Where the pointer is while a press that took hold of this control is still
/// held down. Writes two floats — x and y — to `out` and answers 1, or answers
/// 0 and writes nothing when nothing is being dragged from there.
///
/// Asking is what makes the control a bar: only a press on one the page asked
/// about on the frame it landed on is held, and is a drag rather than a hover
/// until the button comes up. Only a pointer drags; a finger on the same
/// control moves the list instead.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_dragging(
    page: *mut Page<'static>,
    id: c_uint,
    out: *mut c_float,
) -> c_int {
    let Some(at) = page.as_mut().and_then(|page| page.dragging(id)) else {
        return 0;
    };
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(at.as_ptr(), out, 2);
    }
    1
}

/// Where the pointer rests over the page. Writes x and y to `out` and answers
/// 1, or answers 0 and writes nothing: the pointer has left the window, a
/// menu, a dialog or the file chooser stands over the page, a key, a pad or a
/// finger has been reached for since it last moved, or a press that took hold
/// of a bar is held down.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_pointer(page: *const Page<'static>, out: *mut c_float) -> c_int {
    let Some(at) = page.as_ref().and_then(|page| page.pointer()) else {
        return 0;
    };
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(at.as_ptr(), out, 2);
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_pointer_moved(page: *const Page<'static>) -> c_int {
    page.as_ref()
        .map_or(0, |page| c_int::from(page.pointer_moved()))
}

/// The control the pointer moved onto since the last frame, by the number it
/// was drawn with, or -1.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_hovered(page: *const Page<'static>) -> c_int {
    page.as_ref()
        .and_then(|page| page.hovered())
        .map_or(-1, |id| id as c_int)
}

/// A field takes text this frame (`taking` non-zero), standing at `rect`,
/// which may be null. Asked afresh every frame, as the Rust page is; 0 takes
/// back what was said earlier in the same frame.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_taking_text(
    page: *mut Page<'static>,
    taking: c_int,
    rect: *const c_float,
) {
    let Some(page) = page.as_mut() else {
        return;
    };
    page.taking_text(taking != 0);
    if let (true, Some(rect)) = (taking != 0, rectangle(rect)) {
        page.text_at(rect);
    }
}

/// What was typed since the last frame, in order: its letters, and U+0008 for
/// each Backspace. Null for nothing; the caller owns the rest and releases it
/// with `lxb_app_string_free`.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_typed(page: *mut Page<'static>) -> *mut c_char {
    let Some(page) = page.as_mut() else {
        return std::ptr::null_mut();
    };
    let text = spelled(page.typed());
    if text.is_empty() {
        return std::ptr::null_mut();
    }
    owned(&text)
}

/// Where LineXinBar's on-screen keyboard stands over the window: four floats
/// written to `out` and 1, or 0 and nothing written where it is not up.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_board(page: *const Page<'static>, out: *mut c_float) -> c_int {
    let Some(rect) = page.as_ref().and_then(|page| page.shells_board()) else {
        return 0;
    };
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(rect.as_ptr(), out, 4);
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_pressed(page: *mut Page<'static>, id: c_uint) -> c_int {
    match page.as_mut() {
        Some(page) => c_int::from(page.pressed(id)),
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_quit(page: *mut Page<'static>) {
    if let Some(page) = page.as_mut() {
        page.quit();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_keep_awake(
    page: *mut Page<'static>,
    screen: c_int,
    sleep: c_int,
) {
    if let Some(page) = page.as_mut() {
        page.keep_awake(lxb_app::portal::Hold {
            screen: screen != 0,
            sleep: sleep != 0,
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_redraw_within(page: *mut Page<'static>, seconds: c_float) {
    if let (Some(page), true) = (page.as_mut(), seconds.is_finite()) {
        page.redraw_within(std::time::Duration::from_secs_f32(seconds.max(0.0)));
    }
}

/// Whether the toolkit counts this session as low-end hardware: 1 or 0.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_low_end(page: *const Page<'static>) -> c_int {
    page.as_ref().map_or(0, |page| c_int::from(page.low_end()))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_play(page: *mut Page<'static>, sound: Size) {
    if let Some(page) = page.as_mut() {
        page.play(pick(&Sound::ALL, sound));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_volume(page: *mut Page<'static>, value: c_float, muted: c_int) {
    if let Some(page) = page.as_mut() {
        page.set_volume(value, muted != 0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_cursor(page: *const Page<'static>, out: *mut c_float) {
    if let (Some(page), false) = (page.as_ref(), out.is_null()) {
        std::slice::from_raw_parts_mut(out, 4).copy_from_slice(&page.cursor());
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_set_cursor(page: *mut Page<'static>, rect: *const c_float) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.set_cursor(rect);
    }
}

macro_rules! writing {
    ($(#[$doc:meta])* $name:ident => $method:ident) => {
        $(#[$doc])*

        #[no_mangle]
        pub unsafe extern "C" fn $name(page: *mut Page<'static>, text: *const c_char) {
            if let (Some(page), Some(text)) = (page.as_mut(), borrow(text)) {
                page.$method(text);
            }
        }
    };
}

writing!(

    lxb_page_title => title
);
writing!(

    lxb_page_heading => heading
);
writing!(

    lxb_page_text => text
);
writing!(

    lxb_page_note => note
);
writing!(

    lxb_page_icon => icon
);

#[no_mangle]
pub unsafe extern "C" fn lxb_page_head(
    page: *mut Page<'static>,
    icon: *const c_char,
    title: *const c_char,
) {
    if let (Some(page), Some(icon), Some(title)) = (page.as_mut(), borrow(icon), borrow(title)) {
        page.head(icon, title);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_rule(page: *mut Page<'static>) {
    if let Some(page) = page.as_mut() {
        page.rule();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_gap(page: *mut Page<'static>) {
    if let Some(page) = page.as_mut() {
        page.gap();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_button(page: *mut Page<'static>, label: *const c_char) -> c_int {
    match (page.as_mut(), borrow(label)) {
        (Some(page), Some(label)) => c_int::from(page.button(label)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_row(page: *mut Page<'static>, label: *const c_char) -> c_int {
    match (page.as_mut(), borrow(label)) {
        (Some(page), Some(label)) => c_int::from(page.row(label)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_item(page: *mut Page<'static>, label: *const c_char) -> c_int {
    match (page.as_mut(), borrow(label)) {
        (Some(page), Some(label)) => c_int::from(page.item(label)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_row_value(
    page: *mut Page<'static>,
    label: *const c_char,
    value: *const c_char,
) -> c_int {
    match (page.as_mut(), borrow(label), borrow(value)) {
        (Some(page), Some(label), Some(value)) => c_int::from(page.row_value(label, value)),
        _ => 0,
    }
}

/// `rect` is four floats: x, y, width, height.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_light_at(page: *mut Page<'static>, rect: *const c_float) {
    let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) else {
        return;
    };
    page.light_at(rect);
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_menu(
    page: *mut Page<'static>,
    title: *const c_char,
    commands: *const *const c_char,
    count: Size,
) {
    let Some(page) = page.as_mut() else {
        return;
    };
    let commands = strings(commands, count);
    let borrowed: Vec<&str> = commands.iter().map(String::as_str).collect();
    page.menu(borrow(title), &borrowed);
}

/// `marked` is an index into `commands`; anything past the end marks nothing,
/// which is what `lxb_page_menu` passes.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_menu_marked(
    page: *mut Page<'static>,
    title: *const c_char,
    commands: *const *const c_char,
    count: Size,
    marked: Size,
) {
    let Some(page) = page.as_mut() else {
        return;
    };
    let commands = strings(commands, count);
    let borrowed: Vec<&str> = commands.iter().map(String::as_str).collect();
    page.menu_marked(borrow(title), &borrowed, marked as usize);
}

/// A menu of acts, marking nothing, opening with the light on `row`; a row
/// past the end is the first.
#[no_mangle]
pub unsafe extern "C" fn lxb_page_menu_on(
    page: *mut Page<'static>,
    title: *const c_char,
    commands: *const *const c_char,
    count: Size,
    row: Size,
) {
    let Some(page) = page.as_mut() else {
        return;
    };
    let commands = strings(commands, count);
    let borrowed: Vec<&str> = commands.iter().map(String::as_str).collect();
    page.menu_on(borrow(title), &borrowed, row as usize);
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_chose(page: *mut Page<'static>) -> c_int {
    page.as_mut()
        .and_then(Page::chose)
        .map_or(-1, |index| index as c_int)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_ask(
    page: *mut Page<'static>,
    title: *const c_char,
    body: *const c_char,
    answers: *const *const c_char,
    count: Size,
) {
    let Some(page) = page.as_mut() else {
        return;
    };
    let answers = strings(answers, count);
    let borrowed: Vec<&str> = answers.iter().map(String::as_str).collect();
    page.ask(
        borrow(title).unwrap_or_default(),
        borrow(body).unwrap_or_default(),
        &borrowed,
    );
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_answered(page: *mut Page<'static>) -> c_int {
    page.as_mut()
        .and_then(Page::answered)
        .map_or(-1, |index| index as c_int)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_pick(
    page: *mut Page<'static>,
    selection: Size,
    directory: *const c_char,
) -> c_int {
    let Some(selection) = PickerSelection::ALL.get(selection as usize).copied() else {
        return 0;
    };
    match (page.as_mut(), borrow(directory)) {
        (Some(page), Some(directory)) => c_int::from(page.pick(selection, directory)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_pick_many(
    page: *mut Page<'static>,
    selection: Size,
    directory: *const c_char,
) -> c_int {
    let Some(selection) = PickerSelection::ALL.get(selection as usize).copied() else {
        return 0;
    };
    match (page.as_mut(), borrow(directory)) {
        (Some(page), Some(directory)) => c_int::from(page.pick_many(selection, directory)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_save(
    page: *mut Page<'static>,
    name: *const c_char,
    directory: *const c_char,
) -> c_int {
    match (page.as_mut(), borrow(name), borrow(directory)) {
        (Some(page), Some(name), Some(directory)) => c_int::from(page.save(name, directory)),
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_picked(page: *mut Page<'static>) -> *mut c_char {
    page.as_mut()
        .and_then(Page::picked)
        .and_then(|path| path.to_str().map(owned))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_picked_next(page: *mut Page<'static>) -> *mut c_char {
    page.as_mut()
        .and_then(Page::picked_next)
        .and_then(|path| path.to_str().map(owned))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn lxb_app_string_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    drop(CString::from_raw(text));
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_pane(
    page: *mut Page<'static>,
    rect: *const c_float,
    overlay: Size,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().pane(rect, pick(&Overlay::ALL, overlay));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_card(
    page: *mut Page<'static>,
    rect: *const c_float,
    surface: Size,
    role: Size,
    alpha: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().card(
            rect,
            pick(&Surface::ALL, surface),
            pick(&Role::ALL, role),
            alpha,
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_label(
    page: *mut Page<'static>,
    rect: *const c_float,
    text: Size,
    string: *const c_char,
    role: Size,
    align: Size,
) {
    if let (Some(page), Some(rect), Some(string)) = (page.as_mut(), rectangle(rect), borrow(string))
    {
        page.ui().label(
            rect,
            pick(&Text::ALL, text),
            string,
            pick(&Role::ALL, role),
            pick(&[Align::Left, Align::Centre, Align::Right], align),
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_paragraph(
    page: *mut Page<'static>,
    rect: *const c_float,
    string: *const c_char,
    role: Size,
) -> c_float {
    match (page.as_mut(), rectangle(rect), borrow(string)) {
        (Some(page), Some(rect), Some(string)) => {
            page.ui().paragraph(rect, string, pick(&Role::ALL, role))
        }
        _ => 0.0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_icon(
    page: *mut Page<'static>,
    rect: *const c_float,
    name: *const c_char,
) {
    if let (Some(page), Some(rect), Some(name)) = (page.as_mut(), rectangle(rect), borrow(name)) {
        let style = page.icons();
        page.ui().icon(rect, name, style);
    }
}

/// A picture from a file on this disk in `rect`: 1 once it is drawn, 0 while
/// it is still being read, when it cannot be, or without a page, a rectangle
/// or a path. The path is the file system's own bytes, UTF-8 or not.
#[no_mangle]
pub unsafe extern "C" fn lxb_draw_picture(
    page: *mut Page<'static>,
    rect: *const c_float,
    radius: c_float,
    path: *const c_char,
    fit: Size,
    alpha: c_float,
) -> c_int {
    match (page.as_mut(), rectangle(rect), file(path)) {
        (Some(page), Some(rect), Some(path)) => {
            let fit = pick(&[Fit::Contain, Fit::Cover], fit);
            c_int::from(page.ui().picture(rect, radius, path, fit, alpha))
        }
        _ => 0,
    }
}

/// The chosen mark, ringed in dark glass, in the square `rect`.
#[no_mangle]
pub unsafe extern "C" fn lxb_draw_chosen(
    page: *mut Page<'static>,
    rect: *const c_float,
    alpha: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        let style = page.icons();
        page.ui().chosen(rect, style, alpha);
    }
}

/// A switch, on or off, standing in `rect`.
#[no_mangle]
pub unsafe extern "C" fn lxb_draw_switch(
    page: *mut Page<'static>,
    rect: *const c_float,
    on: c_int,
    ink: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        let style = page.icons();
        page.ui().switch(rect, on != 0, style, ink);
    }
}

/// How long a switch `height` tall is.
#[no_mangle]
pub extern "C" fn lxb_switch_width(height: c_float) -> c_float {
    lxb_app::lxb_render::Ui::switch_width(height)
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_rule(page: *mut Page<'static>, rect: *const c_float, role: Size) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().rule(rect, pick(&Role::ALL, role));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_button(
    page: *mut Page<'static>,
    rect: *const c_float,
    label: *const c_char,
    press: Size,
) {
    if let (Some(page), Some(rect), Some(label)) = (page.as_mut(), rectangle(rect), borrow(label)) {
        page.ui().button(rect, label, pressed(press));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_row(
    page: *mut Page<'static>,
    rect: *const c_float,
    name: *const c_char,
    value: *const c_char,
    press: Size,
) {
    if let (Some(page), Some(rect), Some(name)) = (page.as_mut(), rectangle(rect), borrow(name)) {
        page.ui().row(rect, name, borrow(value), pressed(press));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_selection(
    page: *mut Page<'static>,
    rect: *const c_float,
    strength: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().selection(rect, strength);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_draw_spot(page: *mut Page<'static>, id: c_uint, rect: *const c_float) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().spot(id, rect);
    }
}

/// Blur and fade the top and bottom edges of a scrolling area, after
/// everything in it has been drawn.
///
/// `band` is the feather in points; `top` and `bottom` are how strongly each
/// edge is there, which is how a list says whether anything really continues
/// past it.
#[no_mangle]
pub unsafe extern "C" fn lxb_draw_soft_edges(
    page: *mut Page<'static>,
    rect: *const c_float,
    band: c_float,
    top: c_float,
    bottom: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().soft_edges(rect, band, top, bottom);
    }
}

/// Blur and fade the left and right edges of an area, after everything in
/// it has been drawn: what is carried sideways across a pane dissolves into
/// it there.
///
/// `band` is the feather in points; `left` and `right` are how strongly each
/// edge is there.
#[no_mangle]
pub unsafe extern "C" fn lxb_draw_soft_side_edges(
    page: *mut Page<'static>,
    rect: *const c_float,
    band: c_float,
    left: c_float,
    right: c_float,
) {
    if let (Some(page), Some(rect)) = (page.as_mut(), rectangle(rect)) {
        page.ui().soft_side_edges(rect, band, left, right);
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_at(page: *const Page<'static>, x: c_float, y: c_float) -> c_int {
    use lxb_app::lxb_render::Spot;
    match page.as_ref().map(|page| page.at(x, y)) {
        Some(Spot::Control(id)) => id as c_int,
        _ => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_metric(page: *const Page<'static>, metric: Size) -> c_float {
    page.as_ref()
        .map_or(0.0, |page| page.metric(pick(&Metric::ALL, metric)))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_scaled(
    page: *const Page<'static>,
    reference: c_float,
) -> c_float {
    page.as_ref().map_or(0.0, |page| page.scaled(reference))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_line(page: *const Page<'static>, text: Size) -> c_float {
    page.as_ref()
        .map_or(0.0, |page| page.line(pick(&Text::ALL, text)))
}

#[no_mangle]
pub unsafe extern "C" fn lxb_page_measure(
    page: *mut Page<'static>,
    text: Size,
    string: *const c_char,
) -> c_float {
    match (page.as_mut(), borrow(string)) {
        (Some(page), Some(string)) => page.measure(pick(&Text::ALL, text), string),
        _ => 0.0,
    }
}

unsafe fn borrow<'a>(text: *const c_char) -> Option<&'a str> {
    if text.is_null() {
        return None;
    }
    CStr::from_ptr(text).to_str().ok()
}

/// A path as C hands it over: the file system's bytes, which need not be
/// UTF-8.
unsafe fn file<'a>(path: *const c_char) -> Option<&'a Path> {
    if path.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(path).to_bytes();
    Some(Path::new(OsStr::from_bytes(bytes)))
}

/// What was typed, as one string: its letters as they came and U+0008 for each
/// Backspace. A NUL, which C cannot carry, is dropped rather than losing the
/// rest.
fn spelled(typed: impl IntoIterator<Item = Typed>) -> String {
    let mut text = String::new();
    for typed in typed {
        match typed {
            Typed::Wrote(written) => text.extend(written.chars().filter(|&letter| letter != '\0')),
            Typed::Rubbed => text.push('\u{8}'),
        }
    }
    text
}

fn owned(text: &str) -> *mut c_char {
    CString::new(text)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

unsafe fn strings(list: *const *const c_char, count: Size) -> Vec<String> {
    if list.is_null() {
        return Vec::new();
    }
    std::slice::from_raw_parts(list, count as usize)
        .iter()
        .map(|text| borrow(*text).unwrap_or_default().to_string())
        .collect()
}

unsafe fn rectangle(rect: *const c_float) -> Option<[f32; 4]> {
    if rect.is_null() {
        return None;
    }
    let read = std::slice::from_raw_parts(rect, 4);
    Some([read[0], read[1], read[2], read[3]])
}

fn pick<T: Copy>(all: &[T], index: Size) -> T {
    all[(index as usize).min(all.len() - 1)]
}

fn pressed(press: Size) -> Press {
    match press {
        0 => Press::Resting,
        1 => Press::Focused,
        2 => Press::Pressed,
        through => Press::Going(((through - 3) as f32 / 100.0).clamp(0.0, 1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_returned_to_c_is_released_by_this_library() {
        let text = owned("/tmp/selected-file");
        assert!(!text.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(text) }.to_str(),
            Ok("/tmp/selected-file")
        );

        unsafe {
            lxb_app_string_free(text);
            lxb_app_string_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn no_page_hovers_nothing() {
        let mut out = [7.0f32; 2];
        unsafe {
            assert_eq!(lxb_page_pointer(std::ptr::null(), out.as_mut_ptr()), 0);
            assert_eq!(lxb_page_pointer_moved(std::ptr::null()), 0);
            assert_eq!(lxb_page_hovered(std::ptr::null()), -1);
        }
        assert_eq!(out, [7.0; 2], "nothing is written where nothing is known");
    }

    #[test]
    fn no_application_takes_no_picture() {
        let path = c"never.png";
        unsafe {
            assert_eq!(
                lxb_app_shot_after(
                    std::ptr::null_mut(),
                    path.as_ptr(),
                    64,
                    48,
                    6.0,
                    3,
                    None,
                    std::ptr::null_mut()
                ),
                1
            );
        }
    }

    #[test]
    fn an_action_handed_to_no_page_or_past_the_list_does_nothing() {
        unsafe {
            lxb_page_act(std::ptr::null_mut(), 1);
            lxb_page_act(std::ptr::null_mut(), Size::MAX);
        }
    }

    #[test]
    fn no_page_is_not_low_end() {
        assert_eq!(unsafe { lxb_page_low_end(std::ptr::null()) }, 0);
    }

    #[test]
    fn a_soft_side_edge_on_no_page_or_no_rectangle_does_nothing() {
        let rect = [0.0f32, 0.0, 100.0, 50.0];
        unsafe {
            lxb_draw_soft_side_edges(std::ptr::null_mut(), rect.as_ptr(), 10.0, 1.0, 1.0);
            lxb_draw_soft_side_edges(std::ptr::null_mut(), std::ptr::null(), 10.0, 1.0, 1.0);
        }
    }

    #[test]
    fn a_light_in_its_own_shape_on_no_page_writes_nothing() {
        let rect = [0.0f32, 0.0, 100.0, 50.0];
        let mut out = [7.0f32; 4];
        unsafe {
            lxb_page_light_shaped(
                std::ptr::null_mut(),
                rect.as_ptr(),
                12.0,
                1.0,
                1,
                out.as_mut_ptr(),
            );
            lxb_page_light_shaped(
                std::ptr::null_mut(),
                std::ptr::null(),
                12.0,
                1.0,
                0,
                out.as_mut_ptr(),
            );
        }
        assert_eq!(out, [7.0; 4], "nothing is written where nothing is lit");
    }

    #[test]
    fn no_page_takes_no_text_and_has_no_board_over_it() {
        let rect = [10.0f32, 20.0, 300.0, 40.0];
        let mut out = [7.0f32; 4];
        unsafe {
            lxb_page_taking_text(std::ptr::null_mut(), 1, rect.as_ptr());
            lxb_page_taking_text(std::ptr::null_mut(), 0, std::ptr::null());
            assert!(lxb_page_typed(std::ptr::null_mut()).is_null());
            assert_eq!(lxb_page_board(std::ptr::null(), out.as_mut_ptr()), 0);
            assert_eq!(lxb_page_board(std::ptr::null(), std::ptr::null_mut()), 0);
        }
        assert_eq!(out, [7.0; 4], "nothing is written where nothing is known");
    }

    #[test]
    fn what_was_typed_crosses_as_one_string_released_by_this_library() {
        let text = spelled([
            Typed::Wrote("fire".to_string()),
            Typed::Rubbed,
            Typed::Wrote("żółw\0 🙂".to_string()),
        ]);
        assert_eq!(
            text, "fire\u{8}żółw 🙂",
            "a NUL lost the rest of the typing"
        );
        assert!(spelled([]).is_empty());

        let handed = owned(&text);
        assert!(!handed.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(handed) }.to_str(),
            Ok("fire\u{8}żółw 🙂")
        );
        unsafe { lxb_app_string_free(handed) };
    }

    #[test]
    fn a_picture_on_no_page_is_not_drawn() {
        let rect = [0.0f32, 0.0, 100.0, 50.0];
        let path = c"/nowhere/picture.png";
        unsafe {
            assert_eq!(
                lxb_draw_picture(
                    std::ptr::null_mut(),
                    rect.as_ptr(),
                    8.0,
                    path.as_ptr(),
                    0,
                    1.0
                ),
                0
            );
            assert_eq!(
                lxb_draw_picture(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    8.0,
                    path.as_ptr(),
                    1,
                    1.0
                ),
                0
            );
            assert_eq!(
                lxb_draw_picture(
                    std::ptr::null_mut(),
                    rect.as_ptr(),
                    8.0,
                    std::ptr::null(),
                    Size::MAX,
                    1.0
                ),
                0
            );
        }
    }

    #[test]
    fn a_pictures_path_is_the_file_systems_bytes() {
        let path = c"/tmp/\xffpicture.png";
        let read = unsafe { file(path.as_ptr()) }.expect("a path that is not UTF-8");
        assert_eq!(read.as_os_str().as_bytes(), b"/tmp/\xffpicture.png");
        assert!(unsafe { file(std::ptr::null()) }.is_none());
    }
}
