# SG Engine (toy engine)

For now, this is just a toy engine, with many missing features, and breaks every 3 minutes.


### Demo
<a href="http://www.youtube.com/watch?feature=player_embedded&v=w3Lk4ceiyVk " target="_blank"><img src="http://img.youtube.com/vi/w3Lk4ceiyVk/0.jpg" alt="IMAGE ALT TEXT HERE" width="240" height="180" border="10" /></a>

This project is intended to be my first stab at a complete game engine - but it is by no means complete yet. The main ideas around architecture are:

- Modular system design - allowing hot code loading
- composition over inheritance (but it's Rust so of course)

## What works:
- Vulkan rendering using [vulkano](https://github.com/tomaka/vulkano)
- Scene graph (Rc-based ADG) and push-constants 
- Loading obj models using [nom-obj](https://github.com/dwerner/nom-obj)
- Diffuse textures, UVW coordinates

## What doesn't work:
- Multiple textures
- Extremely inefficient shaders (per-vertex matrix operations, for no good reason)
- pretty much anything else... 

As I said, this is very much a work in progress, but I'd love input or PRs or criticism.

## My *opinionated architecture

*We all have our opinions. :)

## Hot loading

To me, on any project that I work on, I like to move quickly, iterate and try ideas fast. I'm a monkey coder, I bash on 
this part and that until I get things to work, and as such, I don't want to wait for a full-compile cycle to try out
a new idea... So a lot of my effort is spent in this library to optimize for that case. I have to give credit to others,
in particular [null program](http://nullprogram.com/blog/2014/12/23/) and therefore indirectly 
[handmade hero](https://handmadehero.org/). I should really get around to watching those videos.



## `game_state`
 
The system is comprised of a base library for shared state, in addition to modules. `game_state` defines all shared,
base types in the system, including the most global `State`, but also shared types and traits defining behavior and
structure of input, ui, events, rendering, world entities, etc.

The `State` struct is central to the design, as it represents the state the game passes between each
module. This allows each module, when not operating on the state, to be reloaded and the old state they are responsible
for to be cleared. The actual loading of the modules is handled in the main project under `src/libloading`.

## Access traits

Several traits are defined and implemented on `State` to serve as a window of responsibility for common operations on
the `State` object itself. This decouples the modules from any exact internal structure of `State`, but also allows
common functionality to be shared between access traits. At a higher level, access traits to `State` serve as a way 
for a mod to state which aspects of `State` it really wants access to.

## Modules
 
Modules are compiled rust code, but are loaded at runtime and can be modified during the course of execution. When a new
version is built, it will be picked up by `libloading` and loaded, while the old library will be unloaded.

In contrast, any changes to the `game_state` crate or it's dependencies (`nom-obj` - an .obj model parser, for instance)
 will need everything to be rebuilt that depends on it, otherwise strange things may happen, or worse.
 
### `mod_asset_loader`

A simple mod intended to load assets and prepare them for use by attaching them to the `State` object.

Access traits used: `RenderLayerAccess`

TODO:
- Expand on asset loading strategy

### `mod_dummy`

This is just a template mod, and is not built or linked, but rather serves as a starting point for creating a new mod.

### `mod_input`

This module is responsible for coordinating and gathering input events in the internal format described in 
`game_state::input`.

Access traits used: `InputAccess`

TODO:
- Gather input from joysticks
- ...

### `mod_rendering`

Responsible for the implementation of renderers, this mod could be split into multiples, adding the capacity for orthogonal
changes to each renderer at runtime. Of course the renderers need to know how to clean themselves up in addition to initialize.

Renderer Status:

- VulkanRenderer - model and texture loading, needs work to expand asset pipeline support
- OpenGLRenderer - Stubbed, little more

Access Traits Used: 
- `RenderAccess`
- `RenderLayerAccess`

TODO:
- Implement OpenGL renderer so this can run on any machine supporting OpenGL
- Renderer specific, but lots of work needs to be done here, probably dependent on `mod_asset_loader` and expansion of
access traits

### `mod_simulation`

Simulation of the game world itself.

Access Traits Used: `SimulationAccess`

Todo:
- everything - this mod is just stubbed at this point

## Notes and Ideas, Links

https://www.youtube.com/watch?v=c1H92b_uLdU

- physics
- network
- audio (supercollider?)
- rendering (vulkan)

## TODO:

- networking
- concurrency model
- simulation
- sim replication
- with futures


## TODOs and notes on networking

### Idea: arbitrary model support through observables (a framework for simulation state sync?)

#### Synchronization model

  http://gafferongames.com/networked-physics/snapshot-compression/

  https://github.com/rygorous/gaffer_net/blob/master/main.cpp

  Target bandwidth range vs Actual bandwidth use

  - separate sync mechanisms for separate properties
  - prioritization of sync for different properties
  - adaptive sync methodology
  - express changing values as rate of change for interp
  - trans Simuation migration of objects
  - support simulation-level persistence (binary file, and maybe redis?)
  - property bounding (limit range, define quantization)
  - custom property serialization traits (e.g. quaternion's 'smallest three')
  - delta compression - send only what has changed
  - arbitrary precision types (like varint)
  - desync handling
  
### Knobs:
  - snapshot send rate (per second)
  - packet size
  - interpolation between snapshots in buffer
  - size of snapshot buffer
  - extrapolation of velocities, linear and angular
  - protocol (tcp/udp) - udp send/ack_send
  - data compression (none, zlib, compress)

### Detections:
  - snapshot length in bytes
  - bandwidth
  - latency
  - packet loss

  Deterministic Lock-step
  Snapshots and Interpolation (send all state)
  State synchronization

  p2p vs client/server
  
  
# Loading sequence
- build in source code within game_state
- default main menu
	- [load world]
		- world file
			- entities
				- chunk 
					- tree( model = tree1.obj, health = 100 )
					- player( model = player1.obj, health = 50 ) 
						- gun( model = gun1.obj )
					- enemy ( model = enemy.obj )
						- gun( model = gun1.obj )
						
						 
					- snake
	- [options]
		- controls
		- sounds
		- graphics
		- updates?
		- mods?

