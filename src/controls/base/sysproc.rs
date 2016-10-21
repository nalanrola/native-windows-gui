/*
    Copyright (C) 2016  Gabriel Dubé

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::mem;
use std::hash::Hash;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use controls::base::{get_handle_data, send_message};
use events::{Event, EventCallback};
use ::constants::{MOD_MOUSE_CTRL, MOD_MOUSE_SHIFT, BTN_MOUSE_MIDDLE, BTN_MOUSE_RIGHT,
 BTN_MOUSE_LEFT, CBN_CLOSEUP, CBN_DROPDOWN, CBN_SETFOCUS, CBN_KILLFOCUS, ControlType,
 CBN_SELCHANGE, STN_CLICKED};

use winapi::{HWND, UINT, WPARAM, LPARAM, LRESULT, WM_USER, WM_SIZING, RECT,
  WM_LBUTTONUP, WM_RBUTTONUP, WM_MBUTTONUP, GET_X_LPARAM, GET_Y_LPARAM,
  WM_COMMAND, HIWORD, BN_CLICKED, BN_SETFOCUS, BN_KILLFOCUS, WM_ACTIVATEAPP,
  UINT_PTR, DWORD_PTR, EN_SETFOCUS, EN_KILLFOCUS, EN_MAXTEXT, EN_CHANGE,
  WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN, WM_SIZE};

use comctl32::{DefSubclassProc};
use user32::{GetWindowRect};

////////////
// Native Windows GUI user events
////////////
pub const NWG_DESTROY_NOTICE: u32 = WM_USER; // Message sent before the actual destruction of a control. Triggers the "removed" event


/**
    Translate a system button event param's
*/
fn handle_btn(msg: UINT, w: WPARAM, l: LPARAM) -> (i32, i32, u32, u32) {
    let (x,y): (i32, i32) = (GET_X_LPARAM(l), GET_Y_LPARAM(l));
    let modifiers = (w as u32) & (MOD_MOUSE_CTRL | MOD_MOUSE_SHIFT);
    let mut btn = (w as u32) & (BTN_MOUSE_MIDDLE | BTN_MOUSE_RIGHT | BTN_MOUSE_LEFT );
    btn |= match msg {
        WM_LBUTTONUP | WM_LBUTTONDOWN => BTN_MOUSE_LEFT,
        WM_RBUTTONUP | WM_RBUTTONDOWN => BTN_MOUSE_RIGHT,
        WM_MBUTTONUP | WM_MBUTTONDOWN => BTN_MOUSE_MIDDLE,
        _ => 0
    };

    (x, y, btn, modifiers)
}

/**
    Get the index and the selected text data of a combobox
*/
fn get_combobox_selection(handle: HWND) -> (u32, String) {
    use winapi::{CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN};
    let selected = send_message(handle, CB_GETCURSEL, 0, 0) as u32;

    let buffer_length = send_message(handle, CB_GETLBTEXTLEN, selected as WPARAM, 0) as usize;
    let mut buffer: Vec<u16> = Vec::with_capacity(buffer_length);
    let ptr: LPARAM;
    unsafe { 
        buffer.set_len(buffer_length); 
        ptr = mem::transmute(buffer.as_mut_ptr());
    }

    send_message(handle, CB_GETLBTEXT, selected as WPARAM, ptr);
    let end_index = buffer.iter().enumerate().find(|&(index, i)| *i == 0).unwrap_or((buffer_length, &0)).0;
    let text = OsString::from_wide(&(buffer.as_slice()[0..end_index]));
    let text = text.into_string().unwrap_or("ERROR!".to_string());

    (selected, text)
}

/**
    Get the sizing rect of a WM_SIZING event
*/
#[inline(always)]
fn handle_sizing(handle: HWND, msg: UINT, rectv: LPARAM) -> (i32, i32, u32, u32) { unsafe {
    let r: RECT = if msg == WM_SIZING {
        (*mem::transmute::<LPARAM, *mut RECT>(rectv)).clone()
    } else if msg == WM_SIZE {
        let mut r = mem::uninitialized();
        GetWindowRect(handle, &mut r);
        r
    } else {
        panic!("Tried to handle sizing event for event ID {}", msg);
    };

    (r.left as i32, r.top as i32, (r.right-r.left) as u32, (r.bottom-r.top) as u32)
}}


/**
    Map system events to application events

    Command ids are not unique, so the type of the control must be passed.
*/
fn map_command<ID: Eq+Hash+Clone>(handle: HWND, evt: UINT, w: WPARAM, l: LPARAM) -> (Vec<Event>, HWND) {
    let command = HIWORD(w as u32);
    let owner: HWND = unsafe{ mem::transmute(l) };
    let data = unsafe{ get_handle_data::<::WindowData<ID>>(owner) };

    if data.is_none() {
        // If somehow, the data of the window was never initialized or was freed
        // Return Event::Unknow because the control is most likely corrupted.
        return (vec![Event::Unknown], handle); 
    }
    
    match data.unwrap()._type {
        ControlType::Button | ControlType::CheckBox | ControlType::GroupBox | ControlType::RadioButton => {
            match command {
                BN_SETFOCUS  | BN_KILLFOCUS => (vec![Event::Focus], owner),
                BN_CLICKED => (vec![Event::Click], owner),
                _ => (vec![Event::Unknown], handle)
            }},
        ControlType::TextInput => {
            match command {
                EN_SETFOCUS  | EN_KILLFOCUS => (vec![Event::Focus], owner),
                EN_CHANGE => (vec![Event::ValueChanged], owner),
                EN_MAXTEXT => (vec![Event::MaxValue], owner),
                _ => (vec![Event::Unknown], handle)
            }},
        ControlType::ComboBox => {
            match command {
                CBN_SETFOCUS  | CBN_KILLFOCUS => (vec![Event::Focus], owner),
                CBN_CLOSEUP => (vec![Event::MenuClose], owner),
                CBN_DROPDOWN => (vec![Event::MenuOpen], owner),
                CBN_SELCHANGE => (vec![Event::ValueChanged, Event::SelectionChanged], owner),
                _ => (vec![Event::Unknown], handle)
            }},
        ControlType::Label => {
            match command {
                STN_CLICKED => (vec![Event::Click], owner),
                _ => (vec![Event::Unknown], handle)
            }},
        _ => (vec![Event::Unknown], handle)
    }
}

/**
    Map system events to application events
*/
#[inline(always)]
fn map_system_event<ID: Eq+Hash+Clone>(handle: HWND, evt: UINT, w: WPARAM, l: LPARAM) -> (Vec<Event>, HWND) {
    match evt {
        WM_COMMAND => map_command::<ID>(handle, evt, w, l), // WM_COMMAND is a special snowflake, it can represent hundreds of different commands
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => (vec![Event::MouseUp], handle),
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => (vec![Event::MouseDown], handle),
        WM_ACTIVATEAPP => (vec![Event::Focus], handle),
        WM_SIZING | WM_SIZE => (vec![Event::Resize], handle),
        NWG_DESTROY_NOTICE => (vec![Event::Removed], handle),
        _ => (vec![Event::Unknown], handle)
    }
}

/**
    Execute an event
*/
#[inline(always)]
fn dispatch_event<ID: Eq+Hash+Clone>(ec: &EventCallback<ID>, ui: &mut ::Ui<ID>, data: &::WindowData<ID>, handle: HWND, msg: UINT, w: WPARAM, l: LPARAM) {
    let caller = &data.id;
    match ec {
        &EventCallback::MouseUp(ref c) | &EventCallback::MouseDown(ref c)  => {
            let (x, y, btn, modifiers) = handle_btn(msg, w, l);
            c(ui, caller, x, y, btn, modifiers); 
        },
        &EventCallback::Click(ref c) | &EventCallback::ValueChanged(ref c) | &EventCallback::MaxValue(ref c) | 
        &EventCallback::Removed(ref c) | &EventCallback::MenuClose(ref c) | &EventCallback::MenuOpen(ref c) => {
            c(ui, caller); 
        },
        &EventCallback::Resize(ref c) => {
            let (x, y, w, h) = handle_sizing(handle, msg, l);
            c(ui, caller, x, y, w, h);
        },
        &EventCallback::Focus(ref c) => {
            let focus = match msg {
                WM_COMMAND => { let w = HIWORD(w as u32); w == BN_SETFOCUS || w == EN_SETFOCUS || w == CBN_SETFOCUS },
                WM_ACTIVATEAPP => w == 1,
                _ => unreachable!()
            };
            c(ui, caller, focus);
        },
        &EventCallback::SelectionChanged(ref c) => {
            let (index, value) = match &data._type {
                &ControlType::ComboBox => get_combobox_selection(handle),
                _ => unreachable!()
            };
            c(ui, caller, index, value);
        }
        //_ => {}
    }
}

#[inline(always)]
pub unsafe fn handle_events<ID: Eq+Hash+Clone>(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) {
    let (events, handle) = map_system_event::<ID>(hwnd, msg, w, l);

    // If the window data was initialized, eval callbacks
    if let Some(data) = get_handle_data::<::WindowData<ID>>(handle) {
        // Build a temporary Ui that is then forgetted to pass it to the callbacks.
        let mut ui = ::Ui{controls: data.controls};

        // Eval the callbacks
        for event in events.iter() {
            if let Some(functions) = data.callbacks.get(&event) {
                for &(_, ref f) in functions.iter() {
                    dispatch_event::<ID>(f, &mut ui, &data, handle, msg, w, l); 
                }
            }
        }
        
        mem::forget(ui);
    }
}

/**
    Window proc for subclassesed native control
*/
pub unsafe extern "system" fn sub_wndproc<ID: Eq+Hash+Clone>(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, id_subclass: UINT_PTR, dref: DWORD_PTR) -> LRESULT {
    handle_events::<ID>(hwnd, msg, w, l);
    return DefSubclassProc(hwnd, msg, w, l);
} 