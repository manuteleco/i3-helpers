# i3-helpers

Assorted set of tools to make my life easier when using i3wm.

Configuration and small support scripts are not included. Only non-trivial
tools.

## Installation

They can be installed with:

```shell
cargo install --git https://github.com/manuteleco/i3-helpers
# or, if you have a local clone
cargo install --path .
```

## Tools

### i3-back-to-scratch

Utility to send windows back to the scratchpad workspace when they lose focus.
Works by connecting to i3 IPC and listening for window focus events. Useful for
*dropdown* terminals.

It should be launched at i3 startup, with a config line like:

```
# Assuming we are setting class="dropdown" on scratchpad windows
exec --no-startup-id i3-back-to-scratch --class dropdown
```
