//! Utility to send windows back to that workspace when they lose focus.
//!
//! This program listens for events from i3 and sends windows that lose focus
//! back to the scratchpad, if their `class` attribute matches the one provided
//! as argument.
//!
//! # Use case
//!
//! This is useful, for example, when using the scratchpad for a terminal
//! application. We can configure i3 to open a terminal window in the scratchpad
//! workspace and use a key binding to toggle its visibility, bringing it to the
//! current workspace in full screen mode and then back again to the scratchpad.
//!
//! However, when the terminal is being displayed, any event that shifts focus
//! to another window can occur, like when we switch to another window or when a
//! new window happens to pop up. The terminal window will then remain visible
//! in the current workspace while also losing its full screen status, which is
//! not what we want. This program solves this problem by sending the terminal
//! back to the scratchpad when it loses focus.

use clap::Parser;
use i3_ipc::{
    event::{Event, Subscribe, WindowChange, WindowData, WorkspaceChange, WorkspaceData},
    reply::Node,
    Connect, I3Stream, I3,
};
use std::io;

/// Send windows back to the scratchpad when they lose focus.
///
/// This program listens for events from i3 and sends windows that lose focus
/// back to the scratchpad, if their `class` attribute matches the one provided
/// as argument.
#[derive(Parser)]
struct Args {
    /// The X11 class of the windows to send back to the scratchpad.
    #[arg(short, long)]
    class: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut focus_monitor = FocusMonitor::new(args.class)?;
    focus_monitor.run()
}

type NodeId = usize;

enum Focused {
    Scratchpad(NodeId),
    Other,
}

pub struct FocusMonitor {
    scratchpad_class: String,
    i3_conn: I3Stream,
    last_focused: Focused,
}

impl FocusMonitor {
    pub fn new(scratchpad_class: String) -> io::Result<Self> {
        Ok(Self {
            scratchpad_class,
            i3_conn: I3::connect()?,
            last_focused: Focused::Other,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        // We need separate connections for listening and sending commands.
        // Otherwise they'll step on each other's toes causing the i3_ipc
        // library to panic when it receives messages it didn't expect.
        let mut i3 = I3Stream::conn_sub([Subscribe::Window, Subscribe::Workspace])?;
        for event in i3.listen() {
            match event? {
                Event::Window(ev) => self.handle_window_event(ev)?,
                Event::Workspace(ev) => self.handle_workspace_event(ev)?,
                _ => unreachable!("Subscribed to only window and workspace events"),
            }
        }
        Ok(())
    }

    fn handle_window_event(&mut self, event: Box<WindowData>) -> io::Result<()> {
        if let WindowChange::Focus = event.change {
            self.handle_last_focused(&event.container)?;
            self.update_last_focused(&event.container);
        }
        Ok(())
    }

    fn handle_workspace_event(&mut self, event: Box<WorkspaceData>) -> io::Result<()> {
        // This branch covers the case when:
        //
        // 1. the scratchpad window is open in a workspace,
        // 2. we switch to another workspace that happens to be empty
        // 3. and then we summon the scratchpad window again.
        //
        // What happens in that, during 2., there are no windows to receive the
        // Window `Focus` event and, therefore, we never send the window to the
        // scratchpad area.
        //
        // This forces us to consider the Workspace `Focus` events as well, and
        // send the window to the scratchpad area when switching to an empty
        // workspace from having the scratchpad window focused.
        if let WorkspaceChange::Focus = event.change {
            let focused_workspace_is_empty = event
                .current
                .as_ref()
                .map(is_empty_workspace)
                .unwrap_or(false);
            if focused_workspace_is_empty {
                if let Focused::Scratchpad(id) = self.last_focused {
                    self.move_to_scratchpad(id)?;
                    self.last_focused = Focused::Other;
                }
            }
        }
        Ok(())
    }

    fn handle_last_focused(&mut self, container: &Node) -> io::Result<()> {
        match self.last_focused {
            Focused::Scratchpad(id) if id != container.id => self.move_to_scratchpad(id)?,
            _ => (),
        }
        Ok(())
    }

    fn update_last_focused(&mut self, container: &Node) {
        self.last_focused = if self.is_scratchpad_window(container) {
            Focused::Scratchpad(container.id)
        } else {
            Focused::Other
        };
    }

    fn is_scratchpad_window(&self, container: &Node) -> bool {
        container
            .window_properties
            .as_ref()
            .and_then(|props| props.class.as_ref())
            .map(|class| class == &self.scratchpad_class)
            .unwrap_or(false)
    }

    fn move_to_scratchpad(&mut self, container_id: usize) -> io::Result<()> {
        let cmd = format!("[con_id={container_id}] move scratchpad");
        self.i3_conn.run_command(&cmd)?;
        Ok(())
    }
}

fn is_empty_workspace(node: &Node) -> bool {
    node.floating_nodes.is_empty() && node.nodes.is_empty()
}
