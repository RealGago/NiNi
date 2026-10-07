---
name: commit
description: Creates commits following Conventional Commits
user_invocable: true
disable_model_invocation: true
---

# Commit

When the user asks you to create a commit:

1. Analyze changes with `git diff` and `git status`
2. Group related changes into a single commit
3. Use Conventional Commits format:
   - `feat:` new feature
   - `fix:` bug fix
   - `docs:` documentation
   - `refactor:` code refactoring
   - `test:` tests
   - `chore:` maintenance
4. Keep the subject line under 72 characters
5. Use imperative mood ("add feature" not "added feature")
6. Add a body if the change needs explanation

## Format
