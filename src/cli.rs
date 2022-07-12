use std::str::FromStr;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use trane::data::filter::FilterOp;

use crate::app::TraneApp;

#[derive(Clone, Debug)]
pub(crate) struct KeyValue {
    pub key: String,
    pub value: String,
}

impl FromStr for KeyValue {
    type Err = anyhow::Error;
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
    #[clap(about = "Add the ccurrent exercise's lesson to the blacklist")]
    Course,

    #[clap(about = "Add the current exercise to the blacklist")]
    Exercise,

    #[clap(about = "Add the current exercise's lesson to the blacklist")]
    Lesson,

    #[clap(about = "Remove unit from the blacklist")]
    Remove {
        #[clap(help = "The unit to remove from the blacklist")]
        unit_id: String,
    },

    #[clap(about = "Show the units currently in the blacklist")]
    Show,

    #[clap(about = "Add the given unit to the blacklist")]
    Unit {
        #[clap(help = "The ID of the unit")]
        unit_id: String,
    },
}

/// Contains subcommands used for debugging.
#[derive(Debug, Subcommand)]
pub(crate) enum DebugSubcommands {
    #[clap(about = "Prints the ID of the unit with the given UID")]
    Id {
        #[clap(help = "The UID of the unit")]
        unit_uid: u64,
    },

    #[clap(about = "Prints the UID of the unit with the given ID")]
    Uid {
        #[clap(help = "The ID of the unit")]
        unit_id: String,
    },

    #[clap(about = "Prints information about the given unit")]
    UnitInfo {
        #[clap(help = "The ID of the unit")]
        unit_id: String,
    },

    #[clap(about = "Prints the type of the unit with the given ID")]
    UnitType {
        #[clap(help = "The ID of the unit")]
        unit_id: String,
    },
}

/// Contains subcommands used for setting and displaying unit filters.
#[derive(Debug, Subcommand)]
pub(crate) enum FilterSubcommands {
    #[clap(about = "Clear the unit filter if any has been set")]
    Clear,

    #[clap(about = "Set the unit filter to only show exercises from the given course")]
    Course {
        #[clap(help = "The ID of the course")]
        ids: Vec<String>,
    },

    #[clap(about = "Set the unit filter to only show exercises from the given lesson")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        ids: Vec<String>,
    },

    #[clap(about = "List the saved unit filters")]
    ListSaved,

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
        #[clap(long = "course-metadata")]
        #[clap(short = 'c')]
        #[clap(multiple_values = true)]
        #[clap(required_unless_present("lesson-metadata"))]
        course_metadata: Option<Vec<KeyValue>>,

        #[clap(help = "Key-value pairs (written as key:value) of lesson metadata to filter on")]
        #[clap(long = "lesson-metadata")]
        #[clap(short = 'l')]
        #[clap(multiple_values = true)]
        #[clap(required_unless_present("course-metadata"))]
        lesson_metadata: Option<Vec<KeyValue>>,
    },

    #[clap(about = "Set the unit filter to the saved filter with the given ID")]
    SetSaved {
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
        course_id: String,
    },

    #[clap(about = "Show the instructions for the given lesson \
        (or the current lesson if none is passed)")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        #[clap(default_value = "")]
        lesson_id: String,
    },
}

/// Contains subcommands used for displaying course and lesson materials.
#[derive(Debug, Subcommand)]
pub(crate) enum MaterialSubcommands {
    #[clap(about = "Show the material for the given course \
        (or the current course if none is passed)")]
    Course {
        #[clap(help = "The ID of the course")]
        #[clap(default_value = "")]
        course_id: String,
    },

    #[clap(about = "Show the material for the given lesson \
        (or the current lesson if none is passed)")]
    Lesson {
        #[clap(help = "The ID of the lesson")]
        #[clap(default_value = "")]
        lesson_id: String,
    },
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

    #[clap(about = "Record the mastery score (1-5) for the current exercise")]
    Score {
        #[clap(help = "The mastery score (1-5) for the current exercise")]
        score: u8,
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
    /// Executes the parsed command.
    pub fn execute_subcommand(&self, app: &mut TraneApp) -> Result<()> {
        match &self.commands {
            Subcommands::Answer => app.show_answer(),

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

            Subcommands::Blacklist(BlacklistSubcommands::Unit { unit_id }) => {
                app.blacklist_unit(unit_id)?;
                println!("Added unit {} to the blacklist", unit_id);
                Ok(())
            }

            Subcommands::Current => app.current(),

            Subcommands::Debug(DebugSubcommands::Id { unit_uid: uid }) => {
                let id = app.get_unit_id(*uid)?;
                println!("The ID for the unit with UID {} is {}", uid, id);
                Ok(())
            }

            Subcommands::Debug(DebugSubcommands::Uid { unit_id: id }) => {
                let uid = app.get_unit_uid(id)?;
                println!("The UID for the unit with ID {} is {}", id, uid);
                Ok(())
            }

            Subcommands::Debug(DebugSubcommands::UnitInfo { unit_id }) => {
                app.show_unit_info(unit_id)
            }

            Subcommands::Debug(DebugSubcommands::UnitType { unit_id: id }) => {
                let unit_type = app.get_unit_type(id)?;
                println!("The type of the unit with ID {} is {:?}", id, unit_type);
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

            Subcommands::Filter(FilterSubcommands::ListSaved) => app.list_filters(),

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

            Subcommands::Filter(FilterSubcommands::SetSaved { id }) => {
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

            Subcommands::Material(MaterialSubcommands::Course { course_id }) => {
                app.show_course_material(course_id)
            }

            Subcommands::Material(MaterialSubcommands::Lesson { lesson_id }) => {
                app.show_lesson_material(lesson_id)
            }

            Subcommands::Next => app.next(),

            Subcommands::Open { library_path } => {
                app.open_library(library_path)?;
                println!("Successfully opened course library at {}", library_path);
                Ok(())
            }

            Subcommands::Score { score } => {
                app.record_score(*score)?;
                println!("Recorded mastery score {} for current exercise", score);
                Ok(())
            }
        }
    }
}
