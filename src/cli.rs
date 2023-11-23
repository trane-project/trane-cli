//! Contains the logic to parse and execute command-line instructions.

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::{path::Path, str::FromStr};
use trane::data::{filter::FilterOp, SchedulerOptions};
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
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum BlacklistSubcommands {
    #[clap(about = "Add the given unit to the blacklist")]
    Add {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "Add the current exercise's course to the blacklist")]
    Course,

    #[clap(about = "Add the current exercise to the blacklist")]
    Exercise,

    #[clap(about = "Add the current exercise's lesson to the blacklist")]
    Lesson,

    #[clap(about = "List the units currently in the blacklist")]
    List,

    #[clap(about = "Remove unit from the blacklist")]
    Remove {
        #[clap(help = "The unit to remove from the blacklist")]
        unit_id: Ustr,
    },

    #[clap(about = "Removes all the units that match the given prefix from the blacklist")]
    RemovePrefix {
        #[clap(help = "The prefix to remove from the blacklist")]
        prefix: String,
    },
}

/// Contains subcommands used for debugging.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum DebugSubcommands {
    #[clap(about = "Exports the dependent graph as a DOT file to the given path")]
    ExportGraph {
        #[clap(help = "The path to the DOT file")]
        path: String,
    },

    #[clap(about = "Trims the storage by removing all trials except for the most recent ones")]
    TrimScores {
        #[clap(help = "The number of trials to keep for each exercise")]
        #[clap(default_value = "20")]
        num_trials: usize,
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

    #[clap(about = "Remove all the trials from units matching the given prefix")]
    RemoveScoresPrefix {
        #[clap(help = "The prefix to match against the trials")]
        prefix: String,
    },
}

/// Contains subcommands used for setting and displaying unit filters.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum FilterSubcommands {
    #[clap(about = "Clear the unit filter if any has been set")]
    Clear,

    #[clap(about = "Set the unit filter to only show exercises from the given courses")]
    Courses {
        #[clap(help = "The IDs of the courses")]
        ids: Vec<Ustr>,
    },

    #[clap(about = "Set the unit filter to only show exercises from the given lessons")]
    Lessons {
        #[clap(help = "The IDs of the lessons")]
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

    #[clap(
        about = "Save the unit filter to only search from the given units and their dependents"
    )]
    Dependents {
        #[clap(help = "The IDs of the units")]
        ids: Vec<Ustr>,
    },

    #[clap(
        about = "Save the unit filter to only search from the given units and their dependencies"
    )]
    Dependencies {
        #[clap(help = "The IDs of the units")]
        ids: Vec<Ustr>,

        #[clap(help = "The maximum depth to search for dependencies")]
        depth: usize,
    },

    #[clap(about = "Select the saved filter with the given ID")]
    Set {
        #[clap(help = "The ID of the saved filter")]
        id: String,
    },

    #[clap(about = "Shows the selected unit filter")]
    Show,
}

/// Contains subcommands used for displaying course and lesson instructions.
#[derive(Clone, Debug, Subcommand)]
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
#[derive(Clone, Debug, Subcommand)]
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
#[derive(Clone, Debug, Subcommand)]
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

/// Contains subcommands used for manipulating git repositories containing Trane courses.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum RepositorySubcommands {
    #[clap(about = "Add a new git repository to the library")]
    Add {
        #[clap(help = "The URL of the git repository")]
        url: String,

        #[clap(help = "The id to assign to the repository")]
        repo_id: Option<String>,
    },

    #[clap(about = "Remove the git repository with the given id from the library")]
    Remove {
        #[clap(help = "The id of the repository")]
        repo_id: String,
    },

    #[clap(about = "List the managed git repositories in the library")]
    List,

    #[clap(about = "Update the managed git repository with the given id")]
    Update {
        #[clap(help = "The id of the repository")]
        repo_id: String,
    },

    #[clap(about = "Update all the managed git repositories in the library")]
    UpdateAll,
}

/// Contains subcommands used for manipulating the review list.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum ReviewListSubcommands {
    #[clap(about = "Add the given unit to the review list")]
    Add {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },

    #[clap(about = "List all the units in the review list")]
    List,

    #[clap(about = "Remove the given unit from the review list")]
    Remove {
        #[clap(help = "The ID of the unit")]
        unit_id: Ustr,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum SchedulerOptionsSubcommands {
    #[clap(about = "Reset the scheduler options to their default values")]
    Reset,

    #[clap(about = "Set the scheduler options to the given values")]
    Set {
        #[clap(help = "The new batch size")]
        #[clap(long = "batch-size")]
        batch_size: usize,
    },

    #[clap(about = "Show the current scheduler options")]
    Show,
}

/// Contains subcommands used for setting and displaying study sessions.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum StudySessionSubcommands {
    #[clap(about = "Clear the study session if any has been set")]
    Clear,

    #[clap(about = "List the saved study sessions")]
    List,

    #[clap(about = "Select the study session with the given ID")]
    Set {
        #[clap(help = "The ID of the saved study session")]
        id: String,
    },

    #[clap(about = "Shows the selected study session")]
    Show,
}
/// Contains the available subcommands.
#[derive(Clone, Debug, Subcommand)]
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

    #[clap(about = "Quit Trane")]
    Quit,

    #[clap(about = "Subcommands for manipulating git repositories containing Trane courses")]
    #[clap(subcommand)]
    Repository(RepositorySubcommands),

    #[clap(about = "Resets the current exercise batch")]
    ResetBatch,

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
        #[clap(short, long, default_value = "20")]
        num_scores: usize,
    },

    #[clap(about = "Subcommands for manipulating the exercise scheduler")]
    #[clap(subcommand)]
    SchedulerOptions(SchedulerOptionsSubcommands),

    #[clap(about = "Subcommands for setting and displaying study sessions")]
    #[clap(subcommand)]
    StudySession(StudySessionSubcommands),
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
    /// Executes the parsed subcommand. Returns true if the application should continue running.
    pub fn execute_subcommand(&self, app: &mut TraneApp) -> Result<bool> {
        match self.commands.clone() {
            Subcommands::Answer => {
                app.show_answer()?;
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::Add { unit_id }) => {
                app.blacklist_unit(unit_id)?;
                println!("Added unit {unit_id} to the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::Course) => {
                app.blacklist_course()?;
                println!("Added current exercise's course to the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::Exercise) => {
                app.blacklist_exercise()?;
                println!("Added current exercise to the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::Lesson) => {
                app.blacklist_lesson()?;
                println!("Added current exercise's lesson to the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::Remove { unit_id }) => {
                app.remove_from_blacklist(unit_id)?;
                println!("Removed {unit_id} from the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::RemovePrefix { prefix }) => {
                app.remove_prefix_from_blacklist(&prefix)?;
                println!("Removed units matching prefix {prefix} from the blacklist");
                Ok(true)
            }

            Subcommands::Blacklist(BlacklistSubcommands::List) => {
                app.list_blacklist()?;
                Ok(true)
            }

            Subcommands::Current => {
                app.current()?;
                Ok(true)
            }

            Subcommands::Debug(DebugSubcommands::ExportGraph { path }) => {
                app.export_graph(Path::new(&path))?;
                println!("Exported graph to {path}");
                Ok(true)
            }

            Subcommands::Debug(DebugSubcommands::TrimScores { num_trials }) => {
                app.trim_scores(num_trials)?;
                Ok(true)
            }

            Subcommands::Debug(DebugSubcommands::UnitInfo { unit_id }) => {
                app.show_unit_info(unit_id)?;
                Ok(true)
            }

            Subcommands::Debug(DebugSubcommands::UnitType { unit_id }) => {
                let unit_type = app.get_unit_type(unit_id)?;
                println!("The type of the unit with ID {unit_id} is {unit_type:?}");
                Ok(true)
            }

            Subcommands::Debug(DebugSubcommands::RemoveScoresPrefix { prefix }) => {
                app.remove_prefix_from_scores(&prefix)?;
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Clear) => {
                app.clear_filter();
                println!("Cleared the unit filter");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Courses { ids }) => {
                app.filter_courses(&ids)?;
                println!("Set the unit filter to only show exercises from the given courses");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Lessons { ids }) => {
                app.filter_lessons(&ids)?;
                println!("Set the unit filter to only show exercises from the given lessons");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::List) => {
                app.list_filters()?;
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Metadata {
                all,
                any,
                lesson_metadata,
                course_metadata,
            }) => {
                let filter_op = match (any, all) {
                    (true, _) => FilterOp::Any,
                    (false, false) | (_, true) => FilterOp::All,
                };
                app.filter_metadata(filter_op, &lesson_metadata, &course_metadata);
                println!("Set the unit filter to only show exercises with the given metadata");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::ReviewList) => {
                app.filter_review_list()?;
                println!("Set the unit filter to only show exercises in the review list");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Dependencies { ids, depth }) => {
                app.filter_dependencies(&ids, depth)?;
                println!(
                    "Set the unit filter to only show exercises starting from the depedents of \
                    the given units"
                );
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Dependents { ids }) => {
                app.filter_dependents(&ids)?;
                println!(
                    "Set the unit filter to only show exercises from the given units and their \
                    dependencies"
                );
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Set { id }) => {
                app.set_filter(&id)?;
                println!("Set the unit filter to the saved filter with ID {id}");
                Ok(true)
            }

            Subcommands::Filter(FilterSubcommands::Show) => {
                app.show_filter();
                Ok(true)
            }

            Subcommands::Instructions(InstructionSubcommands::Course { course_id }) => {
                app.show_course_instructions(course_id)?;
                Ok(true)
            }

            Subcommands::Instructions(InstructionSubcommands::Lesson { lesson_id }) => {
                app.show_lesson_instructions(lesson_id)?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::Courses) => {
                app.list_courses()?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::Dependencies { unit_id }) => {
                app.list_dependencies(unit_id)?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::Dependents { unit_id }) => {
                app.list_dependents(unit_id)?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::Exercises { lesson_id }) => {
                app.list_exercises(lesson_id)?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::Lessons { course_id }) => {
                app.list_lessons(course_id)?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::MatchingCourses) => {
                app.list_matching_courses()?;
                Ok(true)
            }

            Subcommands::List(ListSubcommands::MatchingLessons { course_id }) => {
                app.list_matching_lessons(course_id)?;
                Ok(true)
            }

            Subcommands::Material(MaterialSubcommands::Course { course_id }) => {
                app.show_course_material(course_id)?;
                Ok(true)
            }

            Subcommands::MantraCount => {
                app.show_mantra_count()?;
                Ok(true)
            }

            Subcommands::Material(MaterialSubcommands::Lesson { lesson_id }) => {
                app.show_lesson_material(lesson_id)?;
                Ok(true)
            }

            Subcommands::Next => {
                app.next()?;
                Ok(true)
            }

            Subcommands::Open { library_path } => {
                app.open_library(&library_path)?;
                println!("Successfully opened course library at {library_path}");
                Ok(true)
            }

            Subcommands::Quit => Ok(false),

            Subcommands::Repository(RepositorySubcommands::Add { url, repo_id }) => {
                app.add_repo(&url, repo_id)?;
                println!("Added repository with {url} to the course library");
                Ok(true)
            }

            Subcommands::Repository(RepositorySubcommands::List) => {
                app.list_repos()?;
                Ok(true)
            }

            Subcommands::Repository(RepositorySubcommands::Remove { repo_id }) => {
                app.remove_repo(&repo_id)?;
                println!("Removed repository with ID {repo_id} from the course library.");
                Ok(true)
            }

            Subcommands::Repository(RepositorySubcommands::Update { repo_id }) => {
                app.update_repo(&repo_id)?;
                println!("Updated repository with ID {repo_id}.");
                Ok(true)
            }

            Subcommands::Repository(RepositorySubcommands::UpdateAll) => {
                app.update_all_repos()?;
                println!("Updated all managed repositories.");
                Ok(true)
            }

            Subcommands::ResetBatch => {
                app.reset_batch();
                println!("The exercise batch has been reset.");
                Ok(true)
            }

            Subcommands::ReviewList(ReviewListSubcommands::Add { unit_id }) => {
                app.add_to_review_list(unit_id)?;
                println!("Added unit {unit_id} to the review list.");
                Ok(true)
            }

            Subcommands::ReviewList(ReviewListSubcommands::List) => {
                app.list_review_list()?;
                Ok(true)
            }

            Subcommands::ReviewList(ReviewListSubcommands::Remove { unit_id }) => {
                app.remove_from_review_list(unit_id)?;
                println!("Removed unit {unit_id} from the review list.");
                Ok(true)
            }

            Subcommands::Search { terms } => {
                app.search(&terms)?;
                Ok(true)
            }

            Subcommands::Score { score } => {
                app.record_score(score)?;
                println!("Recorded mastery score {score} for current exercise.");
                Ok(true)
            }

            Subcommands::Scores {
                exercise_id,
                num_scores,
            } => {
                app.show_scores(exercise_id, num_scores)?;
                Ok(true)
            }

            Subcommands::SchedulerOptions(SchedulerOptionsSubcommands::Reset) => {
                app.reset_scheduler_options()?;
                println!("Reset the scheduler options to their default values");
                Ok(true)
            }

            Subcommands::SchedulerOptions(SchedulerOptionsSubcommands::Set { batch_size }) => {
                let options = SchedulerOptions {
                    batch_size,
                    ..Default::default()
                };
                app.set_scheduler_options(options)?;
                println!("Set the batch size to {batch_size}");
                Ok(true)
            }

            Subcommands::SchedulerOptions(SchedulerOptionsSubcommands::Show) => {
                app.show_scheduler_options()?;
                Ok(true)
            }

            Subcommands::StudySession(StudySessionSubcommands::Clear) => {
                app.clear_study_session();
                println!("Cleared the saved study session");
                Ok(true)
            }

            Subcommands::StudySession(StudySessionSubcommands::List) => {
                app.list_study_sessions()?;
                Ok(true)
            }

            Subcommands::StudySession(StudySessionSubcommands::Set { id }) => {
                app.set_study_session(&id)?;
                println!("Set the study session to the saved study session with ID {id}");
                Ok(true)
            }

            Subcommands::StudySession(StudySessionSubcommands::Show) => {
                app.show_study_session();
                Ok(true)
            }
        }
    }
}
