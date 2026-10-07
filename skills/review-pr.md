---
name: review-pr
description: Reviews a Pull Request following team conventions
user_invocable: true
disable_model_invocation: false
---

# Review PR

When the user asks you to review a PR:

1. Read the PR diff using `git diff`
2. Check if commits follow Conventional Commits
3. Check if there are tests for the changes
4. Check if documentation was updated
5. Leave constructive comments, not destructive ones
6. Be concise but thorough

## What to look for

- **Correctness**: Does the code do what it claims?
- **Tests**: Are there tests for new behavior?
- **Edge cases**: Are edge cases handled?
- **Performance**: Any obvious bottlenecks?
- **Security**: Any injection or auth issues?
- **Readability**: Is the code easy to understand?

## Comment style

- ✅ "Suggestion: We could extract this to a function"
- ✅ "Question: What happens if `x` is null here?"
- ❌ "This is wrong"
- ❌ "Why did you do this?"

## Response format
