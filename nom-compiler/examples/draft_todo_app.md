# Todo App

<!-- Draft prose input for `nom author translate`. Run:
       nom author translate examples/draft_todo_app.md --target app
     to see what nomtu + concept proposals the command extracts.
     Add `--write ./data/nomdict.db` to actually materialize them. -->

## Intent

Build a production-ready todo app that lets a user capture a task,
check it off, and see what remains.

## Sketch

- show the list of open tasks
- add a new task by entering text and pressing enter
- mark a task as done by clicking its checkbox
- filter the list to show only active tasks or only completed ones
- persist the list to local storage so tasks survive a page refresh
- show a count of remaining tasks at the bottom
- offer a clear-completed button when at least one task is done

## Nom substitutions

<!-- As you replace each bullet above with a Nom line, paste the Nom
     fragment here for quick diff. The file is ready to rename to
     .nom when every bullet is substituted. -->
