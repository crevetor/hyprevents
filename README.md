Hyprevents emit hyprland events for eww
========================================

This is a small program that uses hyprland's sockets in order to output events that can be used with eww's `deflisten` variable type.

Basically this emits the value read from hyprland when it starts then re-emits the value again every time and event happens that might change the value.


Build
=====

`cargo build`

Usage
=====

Use it in your eww configuration to listen to specific events. For example :

``(deflisten activeworkspace :initial "{}" `/path/to/hyprevents active-workspace`)``