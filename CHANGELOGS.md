# Changelogs

- + => new
- ~ => fix / change
- - => remove

## Last commit: 9005209f2ed516e6a77ad115d2192ef351d1a9b9
+ Added file indexer component
+ Added database error handling
~ Finished file indexer unittests
+ Added file indexer config
+ Added text extractor (unittests + code)

## Last commit: 55842f98914729744914ee774f13296561ea01db
+ Added mmd diagrams (Mermaid diagrams, not MikuMikuDance XD). Via [auto_uml](https://github.com/EthanDGee/Auto-UML)
~ Changed the type of Document's content to Option<String>, so we can not load it when it's not needed
~ Bugfixed, performance improvements

## Last commit: e787fe6c97ef594512b2d453bc6df65666fe095c
+ Added AI component
~ Added error messages (via thiserror)
+ Added improvements on indection

## Last commit: 7eed571d4bb0dbedb3b9ffeae14e98dd716148ad~
- Switched from not_null_check & not_err_check macros to ".?" operator
+ Added MCP server + test (thread-safe)
+ Added multithreading for indexion
~ Switched from extractous to xberg
~ Changed a bunch of other stuff/performance improvements

## Last commit: 1dfda922b9cc0485624a3bbff0c22272947d9996
+ Features usage for cargo / way faster builds
+ Daemon component
+ Use wild for linking for Linux
+ Daemon manager
+ Other smaller improvements

## Last commit: 779ecc745dce28588cb828161c07a523281b457e
+ Code review / applied best practices

## Last commit: a8e75936bd1f7fd1a6d35a531722ed1a6a4bcc4e
~ Finished code-doc for daemon
+ Fixed some bugs
+ Proper logging
+ Small README
+ LOGOOO!

## Last commit: 8606dce4aae6bbff38cd80fe76f111e7172d7499
+ Added settings struct with saving/loading settings to an INI file
+ Unittests for the settings
+ Added controller struct to manage the whole app, (so every component share the same "state")
+ Using the controller instead of Arc in:
  - Daemon
  - MCP
TODO: Unittests for controller
TODO: Re-wrote the manual tests in main

## Last commit: d0eb9d5c1e733de95441d82919d9c78a07830c87
+ Added unittests for controller
+ Improved other unittests for lisibility
+ NO LONGER USING Arc<RwLock\<FileIndexer>> but LockedController for better architecture

## Last commit: 0b1a9cad2ef5022e4c946db542200892c8363d7d
+ Fixed unittests (some were broken)
+ Added unittests for daemon

## Last commit: 0b1a9cad2ef5022e4c946db542200892c8363d7d
+ Started switching the project to a clean architecture, after alot of thinking
+ I'm using a new branch: clean-arch
