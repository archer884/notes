# Notes

For searching and viewing inline comments and definitions in markdown-formatted text files.

## What kind of notes?

Like these:

```markdown
See Spot run. <!-- #Spot is a dog. #character #bio -->
```

Or definitions like this:

```markdown
See Spot run. <!-- a four-legged friend #define:dog -->
```

## How does it work?

The `config` subcommand will configure the program's behavior for your current working directory. Pass it the directory containing your source files.

Notes are parsed and cached in a directory called `.tool`, found in your current working directory (the one you were using when you used the config command). However, the cache is updated each time the modified time for a source file is updated, so you shouldn't need to worry about refreshing it.
