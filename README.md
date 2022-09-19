# trane-cli

This repository contains the code for the command-line interface to
[Trane](https://github.com/trane-project/trane).

## Documentation

The latest documentation for `trane-cli` can be found in the official [Trane
Book](https://trane-project.github.io/trane-cli.html). A copy is shown before for easy reference.

## Installation instructions

[GitHub releases](https://github.com/trane-project/trane-cli/releases) include pre-compiled
binaries. Download the one for your OS and architecture and put it somewhere where you and/or your
shell can find it. There are releases for Linux, Windows, and Mac. Releases for ARM OS X are not
available at the moment because cross-compilation is not working.

## Build instructions

You can also build `trane-cli` from source. The only requirement is an installation of the stable
Rust tool chain. Running `cargo build` from the repository's root should do the job.

You can also run `cargo install` to install the binary in the cargo bin directory.

## Starting guide

### Running the command

To start the binary call `trane`, if you installed it, or `cargo run` from the repo's root
directory. As of now, the binary does not take any arguments. Once you start the CLI, you will
be met with a prompt.

```
trane >>
```

Entering enter executes the input command. Pressing CTRL-C cancels the command. Pressing CTRL-D
sends an EOF signal to break out of the line reading loop.

### Entering your first command

To see the next exercise, enter (prompt not shown for brevity) `trane next`.

Internally, the `clap` library is being used to process the input. This requires that a command name
is present, even though it's redundant because this CLI can only run one command. For this reason,
`trane-cli` automatically prepends the command `trane` if it's not there already. So all commands
can be run without the need for adding `trane` to the beginning.

### Opening a course library

The previous command returns an error because Trane has not opened a course library. A course
library is a set of courses under a directory containing a subdirectory named `.trane/`. Inside this
subdirectory, Trane stores the results of previous exercises, blacklists, and saved filters. This
directory is created automatically.

Let's suppose you have downloaded the [trane-music](https://github.com/trane-project/trane-music)
and called Trane inside that directory. Then, you can type `open ./` to load all the library under
that directory.

### Your first study session

If all the courses are valid, the operation will succeed. Now you can run the next command. Your
first exercise should be shown.

```
trane >> next
Course ID: trane::music::guitar::basic_fretboard
Lesson ID: trane::music::guitar::basic_fretboard::lesson_1
Exercise ID: trane::music::guitar::basic_fretboard::lesson_1::exercise_7

Find the note G in the fretboard at a slow tempo without a metronome.
```

If you are unsure on what to do, you can try looking at the instructions for this lesson by
running the `instructions lesson` command:

```
trane >> instructions lesson
Go down each string and find the given note in the first twelve frets.
Repeat but this time going up the strings.

Do this at a slow tempo but without a metronome.
```

Lessons and courses can also include accompanying material. For example, a lesson on the major scale
could include material defining the major scale, and it's basic intervals for reference. This course
does not contain any material. For those lessons or courses which do, you can display it with the
`material lesson` and `material course` commands respectively.

So this exercise belongs to a course teaching the positions of the notes in the guitar fretboard,
and it is asking us to go up and down the strings to find the note. Once you have given the exercise
a try, you can set your score. There are no objective definitions of which score means but the main
difference between them is the degree of unconscious mastery over the exercise. A score of one means
you are just learning the position of the note, you still make mistakes, and have to commit
conscious effort to the task. A score of five would mean you don't even have to think about the task
because it has thoroughly soaked through all the various pathways involved in learning.

If you want to verify your answer, you can show the answer associated with the current exercise, by
running the `answer` command. Let say we give it a score of two out of five. You can do so by
entering `score 2`. The score is saved, but it's not submitted until you move to the next question
to let you make corrections.

```
trane >> answer
Course ID: trane::music::guitar::basic_fretboard
Lesson ID: trane::music::guitar::basic_fretboard::lesson_1
Exercise ID: trane::music::guitar::basic_fretboard::lesson_1::exercise_7

Answer:

- 1st string (high E): 3rd fret
- 2nd string (B): 8th fret
- 3rd string (G): 12th fret
- 4th string (D): 5th fret
- 5th string (A): 10th fret
- 6th string (low E): 3rd fret
```

To show the current exercise again, you can use the `current` command. Now it's time to move onto
the next question. Questions are presented in the order Trane schedules them and as you master the
exercises you automatically unlock new lessons and courses.

### Short reference for other commands.

At its simplest, the previous commands cover much of the most common operations. The documentation
(accessed with the `help` or `<COMMAND> --help` commands) is pretty much self-explanatory for most
other commands.

```
trane >> help
trane 0.5.0
A command-line interface for Trane

USAGE:
    trane <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    answer          Show the answer to the current exercise, if it exists
    blacklist       Subcommands to manipulate the unit blacklist
    current         Display the current exercise
    debug           Subcommands for debugging purposes
    filter          Subcommands for dealing with unit filters
    help            Print this message or the help of the given subcommand(s)
    instructions    Subcommands for showing course and lesson instructions
    list            Subcommands for listing course, lesson, and exercise IDs
    mantra-count    Show the number of Tara Sarasvati mantras recited in the background during
                        the current session
    material        Subcommands for showing course and lesson materials
    next            Submits the score for the current exercise and proceeds to the next
    open            Open the course library at the given location
    review-list     Subcommands for manipulating the review list
    score           Record the mastery score (1-5) for the current exercise
    scores          Show the most recent scores for the given exercise
```

There are however, some details which warrant further explanation.

The `filter metadata` command allows you to define simple metadata filters. For example, to only
show exercises for the major scale in the key of C, you can type:

```
trane >> filter metadata --course-metadata scale_type:major --lesson-metadata key:C
Set the unit filter to only show exercises with the given metadata
```

The `filter set-saved` command allows you to user more complex filters by storing the definition of
the filter inside the `.trane/filters` directory. For now, a filter can be created by serializing a
struct of type `NamedFilter` into a JSON file (see the file `src/data/filter.rs` inside the Trane
repo for more details). You can refer to those filters by a unique ID in their file, which can be
also shown by running the `filter list-saved` command.
