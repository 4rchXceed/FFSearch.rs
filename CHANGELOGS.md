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
