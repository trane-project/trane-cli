//! Contains the logic to print Trane assets to the terminal.

use anyhow::{Context, Result};
use rand::prelude::SliceRandom;
use std::fs::read_to_string;
use termimad::print_inline;
use trane::data::{
    course_generator::literacy::LiteracyLesson, BasicAsset, ExerciseAsset, ExerciseManifest,
};

/// Prints the markdown file at the given path to the terminal.
pub fn print_markdown(path: &str) -> Result<()> {
    let contents =
        read_to_string(path).with_context(|| format!("Failed to read file at path: {path}"))?;
    print_inline(&contents);
    println!();
    Ok(())
}

/// Randomly samples five values from the given list of strings.
fn sample(values: &[String]) -> Vec<String> {
    let mut sampled = values.to_vec();
    let mut rng = rand::thread_rng();
    sampled.shuffle(&mut rng);
    sampled.truncate(5);
    sampled
}

/// Prints a literacy asset to the terminal.
pub fn print_literacy(lesson_type: &LiteracyLesson, examples: &[String], exceptions: &[String]) {
    let sampled_examples = sample(examples);
    let sampled_exceptions = sample(exceptions);
    match lesson_type {
        LiteracyLesson::Reading => println!("Lesson type: Reading"),
        LiteracyLesson::Dictation => println!("Lesson type: Dictation"),
    }
    if !sampled_examples.is_empty() {
        println!("Examples:");
        for example in sampled_examples {
            print_inline(&example);
            println!();
        }
    }
    if !sampled_exceptions.is_empty() {
        println!("Exceptions:");
        for exception in sampled_exceptions {
            print_inline(&exception);
            println!();
        }
    }
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
            BasicAsset::InlinedAsset { content } => {
                print_inline(content);
                println!();
                Ok(())
            }
            BasicAsset::InlinedUniqueAsset { content } => {
                print_inline(content);
                println!();
                Ok(())
            }
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
            ExerciseAsset::BasicAsset(asset) => asset.display_asset(),
            ExerciseAsset::FlashcardAsset { front_path, .. } => print_markdown(front_path),
            ExerciseAsset::LiteracyAsset {
                lesson_type,
                examples,
                exceptions,
            } => {
                print_literacy(lesson_type, examples, exceptions);
                Ok(())
            }
            ExerciseAsset::SoundSliceAsset {
                link, description, ..
            } => {
                if let Some(description) = description {
                    println!("Exercise description:");
                    print_inline(description);
                    println!();
                }
                println!("SoundSlice link: {link}");
                Ok(())
            }
            ExerciseAsset::TranscriptionAsset { content, .. } => {
                print_inline(content);
                println!();
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
        println!();
        if let Some(description) = &self.description {
            println!("Exercise description: {description}");
            println!();
        }
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
            ExerciseAsset::BasicAsset(_) | ExerciseAsset::TranscriptionAsset { .. } => {
                println!("No answer available for this exercise.");
                println!();
                Ok(())
            }
            ExerciseAsset::FlashcardAsset { back_path, .. } => {
                if let Some(back_path) = back_path {
                    println!("Answer:");
                    println!();
                    print_markdown(back_path)
                } else {
                    println!("No answer available for this exercise.");
                    Ok(())
                }
            }
            ExerciseAsset::SoundSliceAsset { .. } | ExerciseAsset::LiteracyAsset { .. } => Ok(()),
        }
    }
}

impl DisplayAnswer for ExerciseManifest {
    fn display_answer(&self) -> Result<()> {
        println!("Course ID: {}", self.course_id);
        println!("Lesson ID: {}", self.lesson_id);
        println!("Exercise ID: {}", self.id);
        println!();
        self.exercise_asset.display_answer()?;
        Ok(())
    }
}
