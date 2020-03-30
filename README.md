# Quick and ugly GUI (qugui)
> Quickest(?) way to give your ugly bash script an ugly gui

## Quickstart
`qugui path/to/gui-file.yml`

## Development
`RUST_LOG="debug" cargo run examples/basic.yml`

## Examples
#### `examples/basic.yml`
An example that shows off some different features, but doesn't do anything useful.

#### `examples/basic.yml`
An actually useful example where [grim](https://github.com/emersion/grim) is given an ugly gui.

## Reference
#### Configuration file
```yml
title: Window title
# Nodes that build up the gui
nodes:
- Node1 # see Node
- Node2
# How the nodes should be placed
layout:
# One of
# Stack nodes vertically
  Vertical:
    spacing: number
# Stack nodes horizontally
  Horizontal:
    spacing: number
# Fix nodes to a grid
  Grid:
# Actions to run when the app is starting
initialize:
- Action1 # see Action
- Action2
```
#### Node
```yml
# One of
# Button
type: Button
text: Click me!
on_click:
- Action1 # see Action
- Action2
placement: # see Placement
# Will be grayed out if this condition is not met
active_when: # see Conditions

# Radio buttons
type: RadioButtons
# Variable that the selected value should be placed in, see Variables
variable: VARIABLE_NAME
# Options for the different buttons, on the form of value: label
options:
  VALUE0: Label0
  VALUE1: Label1
placement: # see Placement

# Container (used by actions to place new items/nodes in dynamically)
type: Container
# Name of the container, used when referencing the container later
name: container01
placement: # see Placement

# Text input field
type: Input
# Variable to put the text in
variable: VARIABLE_NAME
# Will be grayed out if this condition is not met
active_when: # see Conditions
```
#### Placement
```yml
# How a node should be placed in the window
# If the window layout is horizontal or vertical:
spacing: number
# If the layout is grid
x: number
y: number
w: number
h: number
```
#### Action
```yml
# One of
# Run a command
type: Run
command: ["command", "to", "run"]
# Note that if two Run actions follow each other, stdout from the first will be piped to stdin for the last

# Show stdout (from preceding action) as text in container
# Helpful for debugging
type: Show
container: container_name

# Set a variable, see Variables
type: Var
name: VARIABLE_NAME
value: some value
# Note that the value field is optional, if omitted, the variable will be populated by stdout

# Create a radio button for each line in stdout
type: Options
# Where to store the selected value
variable: VARIABLE_NAME
# What container to put the buttons in
container: container_name

# Display an image in a container
type: Image
# Variable that holds the filename of the image
variable: VARIABLE_NAME
container: container_name
```
#### Variables
Other than the places listed, there are two ways variables affect the program:
1. When a variable is set, it sets an environment variable with the same name and value
2. Instances of the variable name is substituted for the value in commands in run actions
#### Conditions
```yml
# Map of variable: value
VARIABLE: something
# If variable name is suffixed with !, it will be negated
VARIABLE!: something
```
