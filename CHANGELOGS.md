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

## Last commit: 7eed571d4bb0dbedb3b9ffeae14e98dd716148ad
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

## Last commit: f880833aff6ab45ad4a20c5ecf35520b03f4308e
+ Finished to port of the AI part to the clean architecture
+ Tests for file indexer / sync / text extractor
+ Test implementation for AI

## Last commit: 98692120bf272a601f416deb3450a68be68b2309
+ Gitlab CI

## Last commit: 4850bebb40fd4ea553b86f81c710c638594cb15a
+ MCP server migrated
+ Test implementation for the MCP server
+ A file indexer registry with an inmemory implementation

## Last commit: 34c0913b96506fb9c32b9db130f70f9e781026ae
+ Fixed Gitlab CI/CD pipeline: using write_all instead of write
+ Migrated daemon worker
+ Migrated daemon manager
+ Unittests for worker + manager
+ Minor changes

## Last commit: e89d9b6acbfd7c4ebf9463f003dc2e9cb613e646
+ Fixed pipeline (was still broken)
+ Added settings
+ Added unittests for the settings

## Last commit: a2ecdd819e93635ad3b84ce92ad02ea440882eea
+ Started the UI frontend
+ Started coding with iced-xml
+ Finished the system tray
+ Homepage almost finished
+ Task system / visualizer
+ Sync & remove sync folder button
+ Add new sync folder button

## Last commit: 9587047b0cbef08b80f83bff619d524152753803
+ Finished the base settings window

TODO: Recode the system tray, the lib used sucks and doesn't work

## Last commit: 5dd78569a66e8c1183d52eea1f2a466e082a8fd9
+ Replaced the system tray lib with a better one (tray-item)

TODO: Test on Windows

## Last commit: 48c9d5781a7ae40b81d4691099a2f59a44466421
+ Added:
    - Results window
    - Startup window
    - Document details window

+ Bugfixes
+ Gitea Action: https://git.webdash.dev/ArchXceed/FFSearch.rs

## Last commit: 1f42e12a9eba9037bed4fd80519e14ce8d53e8e3
+ Made the CSS / Styling for every window

## Last commit: 4d11dd9bdb4149028bd27f7a0cc421acb7215fc1
Fixed BG + AI UI part: a chat window, some buttons and a new background task

## Last commit: 76b4626a619d026933ee9047cbef7f131b33a203
Added Daemon Manager part: add folders to daemon & start/stop daemon
+ Added Action for "lite version" and "normal version"

## Last commit: b58dddd14ae239097857c49a78fafbb5d025973c
Just a small fix to make de Action pipeline work XD
Errors: https://git.webdash.dev/ArchXceed/FFSearch.rs/actions/runs/26/jobs/58
+ Also fixed Gitlab CI

## Last commit: 97df8addab59a8860600feb3b9ccd63a4b860774
Added MCP button + bugfixes
Added support for path search & binary files

## Last commit: 3288ddcd8f65f25d32e49ad329016eecfc05225b
Fixed bugs
Added "Open" and "Open folder" buttons

## Last commit: cad15c6f7e8e8f6206df6be2fa25c5327ee0e830
Fixed the bug where the program crashes if we opened the UI 2 times

## Last commit: d8a9f61b907c41ad13863df91fb4acf885b9bf48
Added a "search only paths" checkbox for search + window title

## Last commit: f7a7b2250bed8cd97478a3246bde7f9b20b22be1
Polished for the V1.0 release
