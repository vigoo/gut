# gut

My personal and opiniated git/github commands to speed up my usual workflow.

### Implemented commands

#### `work-on <name>`

In any state I indicate that I work on a given named feature.

This performs the following steps:

- if the current branch's name is not `<name>` or `<name>-w{N}` then
  - stash all changes
  - checkout master
  - pull
  - create branch `<name>`
    - if the branch already exists, add a `-w{N}` postfix
  - unstash all
  - run visual merge if necessary
- otherwise nop

#### `work-ready` <message> [jira-id]`

#### `empty-commit`

#### `bump-minor-version`

#### `bump-major-version`

#### `bump-version` 
interactive
