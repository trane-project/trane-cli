//! Contains the logic to parse and execute command-line instructions.

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::{path::Path, str::FromStr};
use trane::data::filter::FilterOp;
use ustr::Ustr;

use crate::app::TraneApp;

/// A key-value pair used to parse course and lesson metadata from the command-line. Pairs are
/// written in the format `<key>:<value>`. Multiple pairs are separated by spaces.
#[derive(Clone, Debug)]
pub(crate) struct KeyValue {
    pub key: String,
    pub value: String,
}

impl FromStr for KeyValue {
    type Err = anyhow::Error;

    /// Parse a string value into a key-value pair.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_value: Vec<&str> = s.trim().split(':').collect();
        if key_value.len() != 2 {
            return Err(anyhow!("Invalid key-value pair"));
        }
        if key_value[0].is_empty() || key_value[1].is_empty() {
            return Err(anyhow!("Invalid key-value pair"));
        }

        Ok(KeyValue {
            key: key_value[0].to_string(),
            value: key_value[1].to_string(),
        })
    }
}

/// Contains subcommands for manipulating the unit blacklist.
#[derive(Debug, Subcommand)]
pub(crate) enum BlacklistSubcommands {
    #[clap(about = "Add the given unit to the blacklist")]
    Add {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Add the ccurrent exercise's lesson to the blacklist")]
    Course,

    #[clap(about = "Add the current exercise to the blacklist")]
    Exercise,

    #[clap(about = "Add the current exercise's lesson to the blacklist")]
    Lesson,

    #[clap(about = "Remove unit from the blacklist")]
    Remove {
        #[clap(help = "The unit to remove from the blacklist")]
        unit_id: Ustr,
    },

    #[clap(about = "Show the units currently in the blacklist")]
    Show,
}

/// Contains subcommands used for debugging.
#[derive(Debug, Subcommand)]
pub(crate) enum DebugSubcommands {
    #[clap(about = "Exports the dependent graph as a DOT file to the given path")]
    ExportGraph {
        #[clap(help = "The path to the DOT file")]
        path: String,
    },

    #[clap(about = "Prints information about the given unit")]
    UnitInfo {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Prints the type of the unit with the given ID")]
    UnitType {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },
}

/// Contains subcommands used for setting and displaying unit filters.
#[derive(Debug, Subcommand)]
pub(crate) enum FilterSubcommands {
    #[clap(about = "Clear the unit filter if any has been set")]
    Clear,

    #[clap(about = "Set the unit filter to only show exercises from the given courses")]
    Course {
        #[clap(help = "The ID of the course")]
        ids: Vec<Ustr>,
    },

    #[clap(about = "Set the unit filter to only show exercises from the given lessons")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        ids: Vec<Ustr>,
    },

    #[clap(about = "List the saved unit filters")]
    List,

    #[clap(about = "Set the unit filter to only show exercises with the given metadata")]
    Metadata {
        #[clap(help = "If true, include units which match all of the key-value pairs")]
        #[clap(long = "all")]
        #[clap(conflicts_with = "any")]
        all: bool,

        #[clap(help = "If true, include units which match any of the key-value pairs")]
        #[clap(long = "any")]
        #[clap(conflicts_with = "all")]
        any: bool,

        #[clap(help = "Key-value pairs (written as key:value) of course metadata to filter on")]
        #[clap(name = "course-metadata")]
        #[clap(long = "course-metadata")]
        #[clap(short = 'c')]
        #[clap(num_args = 1..)]
        #[clap(required_unless_present("lesson-metadata"))]
        course_metadata: Option<Vec<KeyValue>>,

        #[clap(help = "Key-value pairs (written as key:value) of lesson metadata to filter on")]
        #[clap(name = "lesson-metadata")]
        #[clap(long = "lesson-metadata")]
        #[clap(short = 'l')]
        #[clap(num_args = 1..)]
        #[clap(required_unless_present("course-metadata"))]
        lesson_metadata: Option<Vec<KeyValue>>,
    },

    #[clap(about = "Set the unit filter to only show exercises from the units in the review list")]
    ReviewList,

    #[clap(about = "Set the unit filter to the saved filter with the given ID")]
    Set {
        #[clap(help = "The ID of the saved filter")]
        id: String,
    },

    #[clap(about = "Shows the current unit filter")]
    Show,
}

/// Contains subcommands used for displaying course and lesson instructions.
#[derive(Debug, Subcommand)]
pub(crate) enum InstructionSubcommands {
    #[clap(about = "Show the instructions for the given course \
        (or the current course if none is passed)")]
    Course {
        #[clap(help = "The ID of the course")]
        #[clap(default_value = "")]
        course_id: Ustr,
    },

    #[clap(about = "Show the instructions for the given lesson \
        (or the current lesson if none is passed)")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        #[clap(default_value = "")]
        lesson_id: Ustr,
    },
}

/// Contains subcommands used for displaying courses, lessons, and exercises IDs.
#[derive(Debug, Subcommand)]
pub(crate) enum ListSubcommands {
    #[clap(about = "Show the IDs of all courses in the library")]
    Courses,

    #[clap(about = "Show the dependencies of the given unit")]
    Dependencies {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Show the dependents of the given unit")]
    Dependents {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Show the IDs of all exercises in the given lesson")]
    Exercises {
        #[clap(help = "The ID of the lesson")]
        lesson_id: Ustr,
    },

    #[clap(about = "Show the IDs of all lessons in the given course")]
    Lessons {
        #[clap(help = "The ID of the course")]
        course_id: Ustr,
    },

    #[clap(about = "Show the IDs of all the lessons in the given course \
        which match the current filter")]
    MatchingLessons {
        #[clap(help = "The ID of the course")]
        course_id: Ustr,
    },

    #[clap(about = "Show the IDs of all the courses which match the current filter")]
    MatchingCourses,
}

/// Contains subcommands used for displaying course and lesson materials.
#[derive(Debug, Subcommand)]
pub(crate) enum MaterialSubcommands {
    #[clap(about = "Show the material for the given course \
        (or the current course if none is passed)")]
    Course {
        #[clap(help = "The ID of the course")]
        #[clap(default_value = "")]
        course_id: Ustr,
    },

    #[clap(about = "Show the material for the given lesson \
        (or the current lesson if none is passed)")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        #[clap(default_value = "")]
        lesson_id: Ustr,
    },
}

/// Contains subcommands used for manipulating the review list.
#[derive(Debug, Subcommand)]
pub(crate) enum ReviewListSubcommands {
    #[clap(about = "Add the given unit to the review list")]
    Add {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Remove the given unit from the review list")]
    Remove {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Show all the units in the review list")]
    Show,
}

/// Contains the available subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Subcommands {
    #[clap(about = "Show the answer to the current exercise, if it exists")]
    Answer,

    #[clap(about = "Subcommands to manipulate the unit blacklist")]
    #[clap(subcommand)]
    Blacklist(BlacklistSubcommands),

    #[clap(about = "Display the current exercise")]
    Current,

    #[clap(about = "Subcommands for debugging purposes")]
    #[clap(subcommand)]
    Debug(DebugSubcommands),

    #[clap(about = "Subcommands for dealing with unit filters")]
    #[clap(subcommand)]
    Filter(FilterSubcommands),

    #[clap(about = "Subcommands for showing course and lesson instructions")]
    #[clap(subcommand)]
    Instructions(InstructionSubcommands),

    #[clap(about = "Subcommands for listing course, lesson, and exercise IDs")]
    #[clap(subcommand)]
    List(ListSubcommands),

    #[clap(
        about = "Show the number of Tara Sarasvati mantras recited in the background during \
            the current session"
    )]
    #[clap(
        long_about = "Trane \"recites\" Tara Sarasvati's mantra in the background as a symbolic \
            way in which users can contribute back to the Trane Project. This command shows the \
            number of mantras that Trane has recited so far."
    )]
    MantraCount,

    #[clap(about = "Subcommands for showing course and lesson materials")]
    #[clap(subcommand)]
    Material(MaterialSubcommands),

    #[clap(about = "Submits the score for the current exercise and proceeds to the next")]
    Next,

    #[clap(about = "Open the course library at the given location")]
    Open {
        #[clap(help = "The path to the course library")]
        library_path: String,
    },

    #[clap(about = "Subcommands for manipulating the review list")]
    #[clap(subcommand)]
    ReviewList(ReviewListSubcommands),

    #[clap(about = "Record the mastery score (1-5) for the current exercise")]
    Score {
        #[clap(help = "The mastery score (1-5) for the current exercise")]
        score: u8,
    },

    #[clap(about = "Search for courses, lessons, and exercises")]
    Search {
        #[clap(help = "The search query")]
        terms: Vec<String>,
    },

    #[clap(about = "Show the most recent scores for the given exercise")]
    Scores {
        #[clap(help = "The ID of the exercise")]
        #[clap(default_value = "")]
        exercise_id: Ustr,

        #[clap(help = "The number of scores to show")]
        #[clap(default_value = "25")]
        num_scores: usize,
    },
}

/// A command-line interface for Trane.
#[derive(Debug, Parser)]
#[clap(name = "trane")]
#[clap(author, version, about, long_about = None)]
pub(crate) struct TraneCli {
    #[clap(subcommand)]
    pub commands: Subcommands,
}

impl TraneCli {
    /// Executes the parsed subcommand.
    pub fn execute_subcommand(&self, app: &mut TraneApp) -> Result<()> {
        match &self.commands {
            Subcommands::Answer => app.show_answer(),

            Subcommands::Blacklist(BlacklistSubcommands::Add { unit_id }) => {
                app.blacklist_unit(unit_id)?;
                println!("Added unit {} to the blacklist", unit_id);
                Ok(())
            }

            Subcommands::Blacklist(BlacklistSubcommands::Course) => {
                app.blacklist_course()?;
                println!("Added current exercise's course to the blacklist");
                Ok(())
            }

            Subcommands::Blacklist(BlacklistSubcommands::Exercise) => {
                app.blacklist_exercise()?;
                println!("Added current exercise to the blacklist");
                Ok(())
            }

            Subcommands::Blacklist(BlacklistSubcommands::Lesson) => {
                app.blacklist_lesson()?;
                println!("Added current exercise's lesson to the blacklist");
                Ok(())
            }

            Subcommands::Blacklist(BlacklistSubcommands::Remove { unit_id }) => {
                app.whitelist(unit_id)?;
                println!("Removed {} from the blacklist", unit_id);
                Ok(())
            }

            Subcommands::Blacklist(BlacklistSubcommands::Show) => app.show_blacklist(),

            Subcommands::Current => app.current(),

            Subcommands::Debug(DebugSubcommands::ExportGraph { path }) => {
                app.export_graph(Path::new(path))?;
                println!("Exported graph to {}", path);
                Ok(())
            }

            Subcommands::Debug(DebugSubcommands::UnitInfo { unit_id }) => {
                app.show_unit_info(unit_id)
            }

            Subcommands::Debug(DebugSubcommands::UnitType { unit_id }) => {
                let unit_type = app.get_unit_type(unit_id)?;
                println!(
                    "The type of the unit with ID {} is {:?}",
                    unit_id, unit_type
                );
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::Clear) => {
                app.clear_filter();
                println!("Cleared the unit filter");
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::Course { ids }) => {
                app.filter_course(&ids[..])?;
                println!(
                    "Set the unit filter to only show exercises from the course with IDs {:?}",
                    ids
                );
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::Lesson { ids }) => {
                app.filter_lesson(ids)?;
                println!(
                    "Set the unit filter to only show exercises from the lesson with ID {:?}",
                    ids
                );
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::List) => app.list_filters(),

            Subcommands::Filter(FilterSubcommands::Metadata {
                all,
                any,
                lesson_metadata,
                course_metadata,
            }) => {
                let filter_op = match (any, all) {
                    (false, false) => FilterOp::All,
                    (true, _) => FilterOp::Any,
                    (_, true) => FilterOp::All,
                };
                app.filter_metadata(filter_op, lesson_metadata, course_metadata)?;
                println!("Set the unit filter to only show exercises with the given metadata");
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::ReviewList) => {
                app.filter_review_list()?;
                println!("Set the unit filter to only show exercises in the review list");
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::Set { id }) => {
                app.set_filter(id)?;
                println!("Set the unit filter to the saved filter with ID {}", id);
                Ok(())
            }

            Subcommands::Filter(FilterSubcommands::Show) => {
                app.show_filter();
                Ok(())
            }

            Subcommands::Instructions(InstructionSubcommands::Course { course_id }) => {
                app.show_course_instructions(course_id)
            }

            Subcommands::Instructions(InstructionSubcommands::Lesson { lesson_id }) => {
                app.show_lesson_instructions(lesson_id)
            }

            Subcommands::List(ListSubcommands::Courses) => app.list_courses(),

            Subcommands::List(ListSubcommands::Dependencies { unit_id }) => {
                app.list_dependencies(unit_id)
            }

            Subcommands::List(ListSubcommands::Dependents { unit_id }) => {
                app.list_dependents(unit_id)
            }

            Subcommands::List(ListSubcommands::Exercises { lesson_id }) => {
                app.list_exercises(lesson_id)
            }

            Subcommands::List(ListSubcommands::Lessons { course_id }) => {
                app.list_lessons(course_id)
            }

            Subcommands::List(ListSubcommands::MatchingCourses) => app.list_matching_courses(),

            Subcommands::List(ListSubcommands::MatchingLessons { course_id }) => {
                app.list_matching_lessons(course_id)
            }

            Subcommands::Material(MaterialSubcommands::Course { course_id }) => {
                app.show_course_material(course_id)
            }

            Subcommands::MantraCount => app.show_mantra_count(),

            Subcommands::Material(MaterialSubcommands::Lesson { lesson_id }) => {
                app.show_lesson_material(lesson_id)
            }

            Subcommands::Next => app.next(),

            Subcommands::Open { library_path } => {
                app.open_library(library_path)?;
                println!("Successfully opened course library at {}", library_path);
                Ok(())
            }

            Subcommands::ReviewList(ReviewListSubcommands::Add { unit_id }) => {
                app.add_to_review_list(unit_id)?;
                println!("Added unit {} to the review list", unit_id);
                Ok(())
            }

            Subcommands::ReviewList(ReviewListSubcommands::Remove { unit_id }) => {
                app.remove_from_review_list(unit_id)?;
                println!("Removed unit {} from the review list", unit_id);
                Ok(())
            }

            Subcommands::ReviewList(ReviewListSubcommands::Show) => app.show_review_list(),

            Subcommands::Search { terms } => app.search(terms),

            Subcommands::Score { score } => {
                app.record_score(*score)?;
                println!("Recorded mastery score {} for current exercise", score);
                Ok(())
            }

            Subcommands::Scores {
                exercise_id,
                num_scores,
            } => app.show_scores(exercise_id, *num_scores),
        }
    }
}
