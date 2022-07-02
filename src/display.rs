//! Module containing data structures to print assets to the terminal.
use std::fs::read_to_string;

use anyhow::Result;
use termimad::print_inline;
use trane::data::{BasicAsset, ExerciseAsset, ExerciseManifest};

/// Prints the markdown file at the given path to the terminal.
pub fn print_markdown(path: &str) -> Result<()> {
    let contents = read_to_string(path)?;
    print_inline(&contents);
    Ok(())
}

/// Trait to display an asset to the terminal.
pub trait DisplayAsset {
    /// Prints the asset to the terminal.
    fn display_asset(&self) -> Result<()>;
}

impl DisplayAsset for BasicAsset {
    fn display_asset(&self) -> Result<()> {
        match self {
            BasicAsset::MarkdownAsset { path } => print_markdown(path),
        }
    }
}

/// Trait to display an exercise in the terminal.
pub trait DisplayExercise {
    /// Prints the exercise to the terminal.
    fn display_exercise(&self) -> Result<()>;
}

impl DisplayExercise for ExerciseAsset {
    fn display_exercise(&self) -> Result<()> {
        match self {
            ExerciseAsset::FlashcardAsset { front_path, .. } => print_markdown(front_path),
            ExerciseAsset::SoundSliceAsset { link } => {
                println!("SoundSlice link: {}", link);
                Ok(())
            }
        }
    }
}

impl DisplayExercise for ExerciseManifest {
    fn display_exercise(&self) -> Result<()> {
        println!("Course ID: {}", self.course_id);
        println!("Lesson ID: {}", self.lesson_id);
        println!("Exercise ID: {}", self.id);
        if self.description.is_some() {
            println!("Description: {}", self.description.as_ref().unwrap());
        }
        println!();
        self.exercise_asset.display_exercise()?;
        Ok(())
    }
}

/// Trait to display an exercise's answer in the terminal.
pub trait DisplayAnswer {
    /// Prints the exercise's answer to the terminal.
    fn display_answer(&self) -> Result<()>;
}

impl DisplayAnswer for ExerciseAsset {
    fn display_answer(&self) -> Result<()> {
        match self {
            ExerciseAsset::FlashcardAsset { back_path, .. } => print_markdown(back_path),
            ExerciseAsset::SoundSliceAsset { .. } => Ok(()),
        }
    }
}

impl DisplayAnswer for ExerciseManifest {
    fn display_answer(&self) -> Result<()> {
        println!("Course ID: {}", self.course_id);
        println!("Lesson ID: {}", self.lesson_id);
        println!("Exercise ID: {}", self.id);
        println!("Answer:");
        println!();
        self.exercise_asset.display_answer()?;
        Ok(())
    }
}
