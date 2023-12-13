#!/usr/bin/env bash

set -euo pipefail

# Create commit interactively

# conforms commits to https://www.conventionalcommits.org/en/v1.0.0/ standards

# TODO: add support for breaking change

# Check if there are any staged changes
if [[ -z $(git diff --staged --name-only) ]]; then
  gum style --foreground=1 --bold "No staged changes."
  exit 0
fi

# Print the staged files
gum style --foreground=5 "Staged files:"
git diff --staged --name-status

gum confirm "Show full diff?" && git diff --staged

# Runs pre-commit on all staged files
if [ -x "$(command -v pre-commit)" ]; then
  gum style --foreground=5 "Run pre-commit:"
  pre-commit run
fi

gum style --foreground=5 "Configure commit message:"

TYPE=$(gum choose "help" "breaking" "build" "change" "chore" "ci" "deprecate" "docs" "feat" "fix" "perf" "refactor" "remove" "revert" "security" "style" "test")
if [ "$TYPE" = "help" ]; then
  # Descriptions largely taken from: https://medium.com/neudesic-innovation/conventional-commits-a-better-way-78d6785c2e08
  echo "# Commit types
## breaking
A commit that has a footer BREAKING CHANGE:, or appends a ! after the
type/scope, introduces a breaking API change (correlating with MAJOR in
semantic versioning). A BREAKING CHANGE can be part of commits of any type.

## build
Changes that affect the build system or external dependencies (example scopes:
nix, rust)

## change
The commit changes the implementation of an existing feature.

## chore
The commit includes a technical or preventative maintenance task that is
necessary for managing the product or the repository, but it is not tied to any
specific feature or user story. For example, releasing the product can be
considered a chore. Regenerating generated code that must be included in the
repository could be a chore.

## ci
Changes to our CI configuration files and scripts

## deprecate
The commit deprecates existing functionality, but does not remove it from the
product.

## docs
The commit adds, updates, or revises documentation that is stored in the
repository.

## feat
A new feature

## fix
A bug fix

## perf
A code change that improves performance, but not functionality.

## refactor
A code change that neither fixes a bug nor adds a feature

## remove
The commit removes a feature from the product. Typically features are
deprecated first for a period of time before being removed. Removing a feature
from the product may be considered a breaking change that will require a major
version number increment.

## revert
Reverts a previous commit

## security
The commit improves the security of the product or resolves a security issue
that has been reported.

## style
Changes that do not affect the meaning of the code (comments, white-space,
formatting, missing semi-colons, etc)

## test
The commit enhances, adds to, revised, or otherwise changes the suite of
automated tests for the product." | gum format | gum pager

  TYPE=$(gum choose "breaking" "build" "change" "chore" "ci" "deprecate" "docs" "feat" "fix" "perf" "refactor" "remove" "revert" "security" "style" "test")
fi

SCOPE=$(gum input --placeholder "scope")

# Since the scope is optional, wrap it in parentheses if it has a value.
test -n "$SCOPE" && SCOPE="($SCOPE)"

# Pre-populate the input with the type(scope): so that the user may change it
SUMMARY=$(gum input --value "$TYPE$SCOPE: " --placeholder "Summary of this change")
DESCRIPTION=$(gum write --placeholder "Details of this change (CTRL+D to finish)")

# Commit these changes
gum confirm "Commit changes?" && git commit -m "$SUMMARY" -m "$DESCRIPTION"
