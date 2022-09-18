//! Module containing the state of the application.
use std::{fs::File, io::Write, path::Path};

use anyhow::{anyhow, ensure, Result};
use chrono::{Datelike, Local, TimeZone, Utc};
use indoc::formatdoc;
use trane::{
    blacklist::Blacklist,
    course_library::CourseLibrary,
    data::{
        filter::{FilterOp, FilterType, KeyValueFilter, MetadataFilter, UnitFilter},
        ExerciseManifest, MasteryScore, UnitType,
    },
    filter_manager::FilterManager,
    graph::UnitGraph,
    practice_stats::PracticeStats,
    review_list::ReviewList,
    scheduler::ExerciseScheduler,
    Trane,
};
use ustr::Ustr;

use crate::display::{DisplayAnswer, DisplayAsset, DisplayExercise};
use crate::{built_info, cli::KeyValue};

/// Stores the app and its configuration.
#[derive(Default)]
pub(crate) struct TraneApp {
    /// The instance of the Trane library.
    trane: Option<Trane>,

    /// The filter used to select exercises.
    filter: Option<UnitFilter>,

    /// The current batch of exercises.
    batch: Vec<(Ustr, ExerciseManifest)>,

    /// The index of the current exercise in the batch.
    batch_index: usize,

    /// The score given to the current exercise. The score can be changed anytime before the next
    /// exercise is requested.
    current_score: Option<MasteryScore>,
}

impl TraneApp {
    /// Returns the version of the Trane library dependency used by this binary.
    fn trane_version() -> Option<String> {
        for (key, value) in built_info::DEPENDENCIES.iter() {
            if *key == "trane" {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Returns the message shown every time Trane starts up.
    pub fn startup_message() -> String {
        formatdoc! {r#"
                Trane - An automated practice system for learning complex skills
                
                Copyright (C) {} The Trane Project

                This program is free software: you can redistribute it and/or modify
                it under the terms of the GNU General Public License as published by
                the Free Software Foundation, either version 3 of the License, or
                (at your option) any later version.

                This program is distributed in the hope that it will be useful,
                but WITHOUT ANY WARRANTY; without even the implied warranty of
                MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
                GNU General Public License for more details.

                You should have received a copy of the GNU General Public License
                along with this program.  If not, see <https://www.gnu.org/licenses/>.
                
                Trane is dedicated to the memory of John Coltrane and consecrated
                as an offering to the goddess Tārā Sarasvatī.
                
                Oṃ Padmo-yoginī Tāre Tuttāre Ture Prajñā Hrīṃ Hrīṃ Svāhā

                Trane Version: {}
                CLI Version: {}
                Commit Hash: {}

            "#,
            chrono::Utc::now().year(),
            built_info::PKG_VERSION,
            Self::trane_version().unwrap_or("UNKNOWN".to_string()),
            built_info::GIT_COMMIT_HASH.unwrap_or("UNKNOWN"),
        }
        .to_string()
    }

    /// Returns the current exercise.
    fn current_exercise(&self) -> Result<(Ustr, ExerciseManifest)> {
        self.batch
            .get(self.batch_index)
            .cloned()
            .ok_or_else(|| anyhow!("cannot get current exercise"))
    }

    /// Returns the current exercise's course ID.
    fn current_exercise_course(&self) -> Result<Ustr> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let (_, manifest) = self.current_exercise()?;
        Ok(manifest.course_id)
    }

    /// Returns the current exercise's lesson ID.
    fn current_exercise_lesson(&self) -> Result<Ustr> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let (_, manifest) = self.current_exercise()?;
        Ok(manifest.lesson_id)
    }

    /// Submits the score for the current exercise.
    pub fn submit_current_score(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        if let Some(mastery_score) = &self.current_score {
            let curr_exercise = self.current_exercise()?;
            let timestamp = Utc::now().timestamp();
            self.trane.as_ref().unwrap().score_exercise(
                &curr_exercise.0,
                mastery_score.clone(),
                timestamp,
            )?;
        }
        Ok(())
    }

    /// Resets the batch of exercises.
    fn reset_batch(&mut self) {
        // Submit the score for the current exercise but ignore the error because this function
        // might be called before an instance of Trane is open.
        let _ = self.submit_current_score();

        self.batch.clear();
        self.batch_index = 0;
        self.current_score = None;
    }

    /// Returns whether the unit with the given ID exists in the currently opened Trane library.
    fn unit_exists(&self, unit_id: &Ustr) -> Result<bool> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        Ok(self
            .trane
            .as_ref()
            .unwrap()
            .get_unit_type(unit_id)
            .is_some())
    }

    /// Adds the current exercise's course to the blacklist.
    pub fn blacklist_course(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_id = self.current_exercise_course()?;
        self.trane.as_mut().unwrap().add_to_blacklist(&course_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the current exercise's lesson to the blacklist.
    pub fn blacklist_lesson(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.current_exercise_lesson()?;
        self.trane.as_mut().unwrap().add_to_blacklist(&lesson_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the current exercise to the blacklist.
    pub fn blacklist_exercise(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let (_, manifest) = self.current_exercise()?;
        self.trane
            .as_mut()
            .unwrap()
            .add_to_blacklist(&manifest.id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the unit with the given ID to the blacklist.
    pub fn blacklist_unit(&mut self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        ensure!(
            self.unit_exists(unit_id)?,
            "unit {} does not exist",
            unit_id
        );

        self.trane.as_mut().unwrap().add_to_blacklist(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Clears the unit filter if it's set.
    pub fn clear_filter(&mut self) {
        if self.filter.is_none() {
            return;
        }
        self.filter = None;
        self.reset_batch();
    }

    /// Displays the current exercise.
    pub fn current(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let (_, manifest) = self.current_exercise()?;
        manifest.display_exercise()
    }

    /// Returns the given course ID or the current exercise's course ID if the given ID is empty.
    fn course_id_or_current(&self, course_id: &Ustr) -> Result<Ustr> {
        let current_course = self.current_exercise_course().unwrap_or_default();
        if course_id.is_empty() {
            if current_course.is_empty() {
                Err(anyhow!("cannot get current exercise"))
            } else {
                Ok(current_course)
            }
        } else {
            Ok(*course_id)
        }
    }

    /// Returns the given lesson ID or the current exercise's lesson ID if the given ID is empty.
    fn lesson_id_or_current(&self, lesson_id: &Ustr) -> Result<Ustr> {
        let current_lesson = self.current_exercise_lesson().unwrap_or_default();
        if lesson_id.is_empty() {
            if current_lesson.is_empty() {
                Err(anyhow!("cannot get current exercise"))
            } else {
                Ok(current_lesson)
            }
        } else {
            Ok(*lesson_id)
        }
    }

    /// Returns the given exercise ID or the current exercise's ID if the given ID is empty.
    fn exercise_id_or_current(&self, exercise_id: &Ustr) -> Result<Ustr> {
        if exercise_id.is_empty() {
            Ok(self.current_exercise()?.0)
        } else {
            Ok(*exercise_id)
        }
    }

    /// Exports the dependent graph as a DOT file to the given path.
    pub fn export_graph(&self, path: &Path) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let dot_graph = self.trane.as_ref().unwrap().generate_dot_graph();
        let mut file = File::create(path)?;
        file.write_all(dot_graph.as_bytes())?;
        Ok(())
    }

    /// Sets the filter to only show exercises from the given course.
    pub fn filter_course(&mut self, course_ids: &[Ustr]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        for course_id in course_ids {
            let unit_type = self.get_unit_type(course_id)?;
            if unit_type != UnitType::Course {
                return Err(anyhow!("unit with ID {} is not a course", course_id));
            }
        }

        self.filter = Some(UnitFilter::CourseFilter {
            course_ids: course_ids.to_vec(),
        });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises from the given lesson.
    pub fn filter_lesson(&mut self, lesson_ids: &[Ustr]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        for lesson_id in lesson_ids {
            let unit_type = self.get_unit_type(lesson_id)?;
            if unit_type != UnitType::Lesson {
                return Err(anyhow!("unit with ID {} is not a lesson", lesson_id));
            }
        }

        self.filter = Some(UnitFilter::LessonFilter {
            lesson_ids: lesson_ids.to_vec(),
        });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises which belong to any course or lesson with the given
    /// metadata.
    pub fn filter_metadata(
        &mut self,
        filter_op: FilterOp,
        lesson_metadata: &Option<Vec<KeyValue>>,
        course_metadata: &Option<Vec<KeyValue>>,
    ) -> Result<()> {
        let basic_lesson_filters: Option<Vec<KeyValueFilter>> =
            lesson_metadata.as_ref().map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| KeyValueFilter::BasicFilter {
                        key: pair.key.clone(),
                        value: pair.value.clone(),
                        filter_type: FilterType::Include,
                    })
                    .collect()
            });
        let lesson_filter = basic_lesson_filters.map(|filters| KeyValueFilter::CombinedFilter {
            op: filter_op.clone(),
            filters,
        });

        let basic_course_filters: Option<Vec<KeyValueFilter>> =
            course_metadata.as_ref().map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| KeyValueFilter::BasicFilter {
                        key: pair.key.clone(),
                        value: pair.value.clone(),
                        filter_type: FilterType::Include,
                    })
                    .collect()
            });
        let course_filter = basic_course_filters.map(|filters| KeyValueFilter::CombinedFilter {
            op: filter_op.clone(),
            filters,
        });

        self.filter = Some(UnitFilter::MetadataFilter {
            filter: MetadataFilter {
                op: filter_op,
                lesson_filter,
                course_filter,
            },
        });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises from the review list.
    pub fn filter_review_list(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.filter = Some(UnitFilter::ReviewListFilter);
        self.reset_batch();
        Ok(())
    }

    /// Returns the type of the unit with the given ID.
    pub fn get_unit_type(&self, unit_id: &Ustr) -> Result<UnitType> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_ref()
            .unwrap()
            .get_unit_type(unit_id)
            .ok_or_else(|| anyhow!("missing type for unit with ID {}", unit_id))
    }

    /// Prints the list of all the saved unit filters.
    pub fn list_filters(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let filters = self.trane.as_ref().unwrap().list_filters();

        if filters.is_empty() {
            println!("No saved unit filters");
            return Ok(());
        }

        println!("Saved unit filters:");
        println!("ID\tDescription");
        for filter in filters {
            println!("{}\t{}", filter.0, filter.1);
        }
        Ok(())
    }

    /// Lists the IDs of all the courses in the library.
    pub fn list_courses(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let courses = self.trane.as_ref().unwrap().get_course_ids();
        if courses.is_empty() {
            println!("No courses in library");
            return Ok(());
        }

        println!("Courses:");
        println!();
        for course in courses {
            println!("{}", course);
        }
        Ok(())
    }

    /// Lists the dependencies of the given unit.
    pub fn list_dependencies(&self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        if unit_type == UnitType::Exercise {
            return Err(anyhow!("Exercises do not have dependencies"));
        }

        let dependencies = self
            .trane
            .as_ref()
            .unwrap()
            .get_dependencies(unit_id)
            .unwrap_or_default();
        if dependencies.is_empty() {
            println!("No dependencies for unit with ID {}", unit_id);
            return Ok(());
        }

        println!("Dependencies:");
        println!();
        for dependency in dependencies {
            println!("{}", dependency);
        }
        Ok(())
    }

    /// Lists the dependents of the given unit.
    pub fn list_dependents(&self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        if unit_type == UnitType::Exercise {
            return Err(anyhow!("Exercises do not have dependents"));
        }

        let dependents = self
            .trane
            .as_ref()
            .unwrap()
            .get_dependents(unit_id)
            .unwrap_or_default();
        if dependents.is_empty() {
            println!("No dependents for unit with ID {}", unit_id);
            return Ok(());
        }

        println!("Dependents:");
        println!();
        for dependent in dependents {
            println!("{}", dependent);
        }
        Ok(())
    }

    /// Lists the IDs of all the exercises in the given lesson.
    pub fn list_exercises(&self, lesson_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let exercises = self.trane.as_ref().unwrap().get_exercise_ids(lesson_id)?;
        if exercises.is_empty() {
            println!("No exercises in lesson {}", lesson_id);
            return Ok(());
        }

        println!("Exercises:");
        println!();
        for exercise in exercises {
            println!("{}", exercise);
        }
        Ok(())
    }

    /// Lists the IDs of all the lessons in the given course.
    pub fn list_lessons(&self, course_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lessons = self.trane.as_ref().unwrap().get_lesson_ids(course_id)?;
        if lessons.is_empty() {
            println!("No lessons in course {}", course_id);
            return Ok(());
        }

        println!("Lessons:");
        println!();
        for lesson in lessons {
            println!("{}", lesson);
        }
        Ok(())
    }

    /// Lists all the courses which match the current filter.
    pub fn list_matching_courses(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let courses: Vec<Ustr> = self
            .trane
            .as_ref()
            .unwrap()
            .get_course_ids()
            .into_iter()
            .filter(|course_id| {
                if self.filter.is_none() {
                    return true;
                }

                let filter = self.filter.as_ref().unwrap();
                let manifest = self.trane.as_ref().unwrap().get_course_manifest(course_id);
                match manifest {
                    Some(manifest) => match filter {
                        UnitFilter::CourseFilter { .. } => filter.apply_course_id(course_id),
                        UnitFilter::LessonFilter { .. } => false,
                        UnitFilter::MetadataFilter { .. } => {
                            filter.apply_course_metadata(&manifest)
                        }
                        UnitFilter::ReviewListFilter => false,
                    },
                    None => false,
                }
            })
            .collect();

        if courses.is_empty() {
            println!("No matching courses");
            return Ok(());
        }

        println!("Matching courses:");
        println!();
        for course in courses {
            println!("{}", course);
        }
        Ok(())
    }

    /// Lists all the lessons in the given course which match the current filter.
    pub fn list_matching_lessons(&self, course_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lessons: Vec<Ustr> = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_ids(course_id)?
            .into_iter()
            .filter(|lesson_id| {
                if self.filter.is_none() {
                    return true;
                }

                let filter = self.filter.as_ref().unwrap();
                let lesson_manifest = self.trane.as_ref().unwrap().get_lesson_manifest(lesson_id);
                match lesson_manifest {
                    Some(lesson_manifest) => match filter {
                        UnitFilter::CourseFilter { .. } => {
                            filter.apply_course_id(&lesson_manifest.course_id)
                        }
                        UnitFilter::LessonFilter { .. } => filter.apply_lesson_id(lesson_id),
                        UnitFilter::MetadataFilter { .. } => {
                            let course_manifest = self
                                .trane
                                .as_ref()
                                .unwrap()
                                .get_course_manifest(&lesson_manifest.course_id);
                            if course_manifest.is_none() {
                                // This should never happen but print the lesson ID if it does.
                                return true;
                            }
                            let course_manifest = course_manifest.unwrap();
                            filter.apply_lesson_metadata(&lesson_manifest, &course_manifest)
                        }
                        UnitFilter::ReviewListFilter => false,
                    },
                    None => false,
                }
            })
            .collect();

        if lessons.is_empty() {
            println!("No matching lessons in course {}", course_id);
            return Ok(());
        }

        println!("Lessons:");
        println!();
        for lesson in lessons {
            println!("{}", lesson);
        }
        Ok(())
    }

    /// Displays the next exercise.
    pub fn next(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        // Submit the current score before moving on to the next exercise.
        self.submit_current_score()?;

        self.current_score = None;
        self.batch_index += 1;
        if self.batch.is_empty() || self.batch_index >= self.batch.len() {
            self.batch = self
                .trane
                .as_ref()
                .unwrap()
                .get_exercise_batch(self.filter.as_ref())?;
            self.batch_index = 0;
        }

        let (_, manifest) = self.current_exercise()?;
        manifest.display_exercise()
    }

    /// Opens the course library at the given path.
    pub fn open_library(&mut self, library_root: &str) -> Result<()> {
        let trane = Trane::new(Path::new(library_root))?;
        self.trane = Some(trane);
        self.batch.drain(..);
        self.batch_index = 0;
        Ok(())
    }

    /// Assigns the given score to the current exercise.
    pub fn record_score(&mut self, score: u8) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let mastery_score = match score {
            1 => Ok(MasteryScore::One),
            2 => Ok(MasteryScore::Two),
            3 => Ok(MasteryScore::Three),
            4 => Ok(MasteryScore::Four),
            5 => Ok(MasteryScore::Five),
            _ => Err(anyhow!("invalid score {}", score)),
        }?;
        self.current_score = Some(mastery_score);
        Ok(())
    }

    /// Sets the unit filter to the saved filter with the given ID.
    pub fn set_filter(&mut self, filter_id: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let named_filter = self
            .trane
            .as_ref()
            .unwrap()
            .get_filter(filter_id)
            .ok_or_else(|| anyhow!("no filter with ID {}", filter_id))?;
        self.filter = Some(named_filter.filter);
        Ok(())
    }

    /// Shows the answer to the current exercise.
    pub fn show_answer(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let curr_exercise = self.current_exercise()?;
        curr_exercise.1.display_answer()
    }

    /// Shows all the entries in the blacklist.
    pub fn show_blacklist(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let entries = self.trane.as_ref().unwrap().all_blacklist_entries()?;

        if entries.is_empty() {
            println!("No entries in the blacklist");
            return Ok(());
        }
        println!("Blacklist:");
        for entry in entries {
            println!("Unit ID: {}", entry);
        }
        Ok(())
    }

    /// Shows the currently set filter.
    pub fn show_filter(&self) {
        if self.filter.is_none() {
            println!("No filter is set");
        } else {
            println!("Filter:");
            println!("{:#?}", self.filter.as_ref().unwrap());
        }
    }

    /// Shows the course instructions for the given course.
    pub fn show_course_instructions(&self, course_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_id = self.course_id_or_current(course_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_course_manifest(&course_id)
            .ok_or_else(|| anyhow!("no manifest for course with ID {}", course_id))?;
        match manifest.course_instructions {
            None => {
                println!("Course has no instructions");
                Ok(())
            }
            Some(instructions) => instructions.display_asset(),
        }
    }

    /// Shows the lesson instructions for the given lesson.
    pub fn show_lesson_instructions(&self, lesson_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.lesson_id_or_current(lesson_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_manifest(&lesson_id)
            .ok_or_else(|| anyhow!("no manifest for lesson with ID {}", lesson_id))?;
        match manifest.lesson_instructions {
            None => {
                println!("Lesson has no instructions");
                Ok(())
            }
            Some(instructions) => instructions.display_asset(),
        }
    }

    /// Shows the course material for the given course.
    pub fn show_course_material(&self, course_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_id = self.course_id_or_current(course_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_course_manifest(&course_id)
            .ok_or_else(|| anyhow!("no manifest for course with ID {}", course_id))?;
        match manifest.course_material {
            None => {
                println!("Course has no material");
                Ok(())
            }
            Some(material) => material.display_asset(),
        }
    }

    /// Shows the lesson material for the given lesson.
    pub fn show_lesson_material(&self, lesson_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.lesson_id_or_current(lesson_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_manifest(&lesson_id)
            .ok_or_else(|| anyhow!("no manifest for lesson with ID {}", lesson_id))?;
        match manifest.lesson_material {
            None => {
                println!("Lesson has no material");
                Ok(())
            }
            Some(material) => material.display_asset(),
        }
    }

    /// Shows the current count of
    pub fn show_mantra_count(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        println!(
            "Mantra count: {}",
            self.trane.as_ref().unwrap().mantra_count()
        );
        Ok(())
    }

    /// Shows the most recent scores for the given exercise.
    pub fn show_scores(&self, exercise_id: &Ustr, num_scores: usize) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let exercise_id = self.exercise_id_or_current(exercise_id)?;
        let scores = self
            .trane
            .as_ref()
            .unwrap()
            .get_scores(&exercise_id, num_scores)?;

        println!("Scores for exercise \"{}\":", exercise_id);
        println!("{:<20} Score", "Date");
        println!();
        for score in scores {
            let dt = Local.timestamp(score.timestamp, 0);
            println!(
                "{} {:>6}",
                dt.format("%Y-%m-%d %H:%M:%S"),
                score.score as u8
            );
        }
        Ok(())
    }

    /// Prints the manifest for the unit with the given UID.
    fn show_unit_manifest(&self, unit_id: &Ustr, unit_type: UnitType) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        match unit_type {
            UnitType::Exercise => {
                let manifest = self
                    .trane
                    .as_ref()
                    .unwrap()
                    .get_exercise_manifest(unit_id)
                    .ok_or_else(|| anyhow!("missing manifest for exercise {}", unit_id))?;
                println!("Unit manifest:");
                println!("{:#?}", manifest);
            }
            UnitType::Lesson => {
                let manifest = self
                    .trane
                    .as_ref()
                    .unwrap()
                    .get_lesson_manifest(unit_id)
                    .ok_or_else(|| anyhow!("missing manifest for lesson {}", unit_id))?;
                println!("Unit manifest:");
                println!("{:#?}", manifest);
            }
            UnitType::Course => {
                let manifest = self
                    .trane
                    .as_ref()
                    .unwrap()
                    .get_course_manifest(unit_id)
                    .ok_or_else(|| anyhow!("missing manifest for course {}", unit_id))?;
                println!("Unit manifest:");
                println!("{:#?}", manifest);
            }
        };
        Ok(())
    }

    /// Prints information about the given unit.
    pub fn show_unit_info(&self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        println!("Unit ID: {}", unit_id);
        println!("Unit type: {:?}", unit_type);
        self.show_unit_manifest(unit_id, unit_type)
    }

    /// Removes the given unit from the blacklist.
    pub fn whitelist(&mut self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_mut()
            .unwrap()
            .remove_from_blacklist(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the given unit to the review list.
    pub fn add_to_review_list(&mut self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        ensure!(
            self.unit_exists(unit_id)?,
            "unit {} does not exist",
            unit_id
        );

        self.trane.as_mut().unwrap().add_to_review_list(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Removes the given unit from the review list.
    pub fn remove_from_review_list(&mut self, unit_id: &Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_mut()
            .unwrap()
            .remove_from_review_list(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Shows all the units in the review list.
    pub fn show_review_list(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let entries = self.trane.as_ref().unwrap().all_review_list_entries()?;
        if entries.is_empty() {
            println!("No entries in the blacklist");
            return Ok(());
        }

        println!("Review list:");
        println!("{:<50} {:<10}", "Unit ID", "Unit type");
        for unit_id in entries {
            let unit_type = self.get_unit_type(&unit_id);
            if unit_type.is_err() {
                println!("{:<50} {:<10}", unit_id.as_str(), "Unknown");
            } else {
                println!("{:<50} {:#<10?}", unit_id.as_str(), unit_type.unwrap());
            }
        }
        Ok(())
    }
}
