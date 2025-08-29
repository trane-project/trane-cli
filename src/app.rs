//! Contains the state of the application and the logic to interact with Trane.

use anyhow::{anyhow, bail, ensure, Result};
use chrono::{Datelike, Local, TimeZone, Utc};
use indoc::formatdoc;
use std::{fs::File, io::Write, path::Path};
use trane::{
    blacklist::Blacklist,
    course_library::CourseLibrary,
    data::{
        filter::{
            ExerciseFilter, FilterOp, FilterType, KeyValueFilter, StudySessionData, UnitFilter,
        },
        ExerciseManifest, MasteryScore, SchedulerOptions, UnitType,
    },
    exercise_scorer::{ExerciseScorer, ExponentialDecayScorer},
    filter_manager::FilterManager,
    graph::UnitGraph,
    practice_rewards::PracticeRewards,
    practice_stats::PracticeStats,
    repository_manager::RepositoryManager,
    review_list::ReviewList,
    reward_scorer::{RewardScorer, WeightedRewardScorer},
    scheduler::ExerciseScheduler,
    study_session_manager::StudySessionManager,
    transcription_downloader::TranscriptionDownloader,
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

    /// The study session used to select exercises.
    study_session: Option<StudySessionData>,

    /// The current batch of exercises.
    batch: Vec<ExerciseManifest>,

    /// The index of the current exercise in the batch.
    batch_index: usize,

    /// The score given to the current exercise. The score can be changed anytime before the next
    /// exercise is requested.
    current_score: Option<MasteryScore>,
}

impl TraneApp {
    /// Returns the version of the Trane library dependency used by this binary.
    fn trane_version() -> Option<String> {
        for (key, value) in &built_info::DEPENDENCIES {
            if *key == "trane" {
                return Some((*value).to_string());
            }
        }
        None
    }

    /// Returns the message shown every time Trane starts up.
    pub fn startup_message() -> String {
        formatdoc! {r#"
                Trane - An automated practice system for learning complex skills
                
                Copyright (C) 2022 - {} The Trane Project

                This program is free software: you can redistribute it and/or modify
                it under the terms of the GNU Affero General Public License as
                published by the Free Software Foundation, either version 3 of the
                License, or (at your option) any later version.

                This program is distributed in the hope that it will be useful,
                but WITHOUT ANY WARRANTY; without even the implied warranty of
                MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
                GNU Affero General Public License for more details.

                You should have received a copy of the GNU Affero General Public License
                along with this program.  If not, see <https://www.gnu.org/licenses/>.
                
                Trane is named after John Coltrane and dedicated to his memory. The
                liner notes for "A Love Supreme" are reproduced below. May this project
                be too such an offering.
                
                > This album is a humble offering to Him. An attempt to say "THANK
                > YOU GOD" through our work, even as we do in our hearts and with our
                > tongues. May He help and strengthen all men in every good endeavor.
                
                Trane Version: {}
                CLI Version: {}
                Commit Hash: {}

            "#,
            chrono::Utc::now().year(),
            Self::trane_version().unwrap_or_else(|| "UNKNOWN".to_string()),
            built_info::PKG_VERSION,
            built_info::GIT_COMMIT_HASH.unwrap_or("UNKNOWN"),
        }
    }

    /// Returns the current exercise.
    fn current_exercise(&self) -> Result<ExerciseManifest> {
        self.batch
            .get(self.batch_index)
            .cloned()
            .ok_or_else(|| anyhow!("cannot get current exercise"))
    }

    /// Returns the current exercise's course ID.
    fn current_exercise_course(&self) -> Result<Ustr> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let manifest = self.current_exercise()?;
        Ok(manifest.course_id)
    }

    /// Returns the current exercise's lesson ID.
    fn current_exercise_lesson(&self) -> Result<Ustr> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let manifest = self.current_exercise()?;
        Ok(manifest.lesson_id)
    }

    /// Submits the score for the current exercise.
    pub fn submit_current_score(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        if let Some(mastery_score) = &self.current_score {
            let curr_exercise = self.current_exercise()?;
            let timestamp = Utc::now().timestamp();
            self.trane.as_ref().unwrap().score_exercise(
                curr_exercise.id,
                mastery_score.clone(),
                timestamp,
            )?;
        }
        Ok(())
    }

    /// Resets the batch of exercises.
    pub fn reset_batch(&mut self) {
        // Submit the score for the current exercise but ignore the error because this function
        // might be called before an instance of Trane is open.
        let _ = self.submit_current_score();

        self.batch.clear();
        self.batch_index = 0;
        self.current_score = None;
    }

    /// Returns whether the unit with the given ID exists in the currently opened Trane library.
    fn unit_exists(&self, unit_id: Ustr) -> Result<bool> {
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
        self.trane.as_mut().unwrap().add_to_blacklist(course_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the current exercise's lesson to the blacklist.
    pub fn blacklist_lesson(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.current_exercise_lesson()?;
        self.trane.as_mut().unwrap().add_to_blacklist(lesson_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the current exercise to the blacklist.
    pub fn blacklist_exercise(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let manifest = self.current_exercise()?;
        self.trane.as_mut().unwrap().add_to_blacklist(manifest.id)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds the unit with the given ID to the blacklist.
    pub fn blacklist_unit(&mut self, unit_id: Ustr) -> Result<()> {
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
        self.study_session = None;
        self.reset_batch();
    }

    /// Displays the current exercise.
    pub fn current(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let manifest = self.current_exercise()?;
        manifest.display_exercise()
    }

    /// Returns the given course ID or the current exercise's course ID if the given ID is empty.
    fn course_id_or_current(&self, course_id: Ustr) -> Result<Ustr> {
        let current_course = self.current_exercise_course().unwrap_or_default();
        if course_id.is_empty() {
            if current_course.is_empty() {
                Err(anyhow!("cannot get current exercise"))
            } else {
                Ok(current_course)
            }
        } else {
            Ok(course_id)
        }
    }

    /// Returns the given lesson ID or the current exercise's lesson ID if the given ID is empty.
    fn lesson_id_or_current(&self, lesson_id: Ustr) -> Result<Ustr> {
        let current_lesson = self.current_exercise_lesson().unwrap_or_default();
        if lesson_id.is_empty() {
            if current_lesson.is_empty() {
                Err(anyhow!("cannot get current exercise"))
            } else {
                Ok(current_lesson)
            }
        } else {
            Ok(lesson_id)
        }
    }

    /// Returns the given exercise ID or the current exercise's ID if the given ID is empty.
    fn exercise_id_or_current(&self, exercise_id: Ustr) -> Result<Ustr> {
        if exercise_id.is_empty() {
            Ok(self.current_exercise()?.id)
        } else {
            Ok(exercise_id)
        }
    }

    /// Exports the dependent graph as a DOT file to the given path.
    pub fn export_graph(&self, path: &Path, courses_only: bool) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let dot_graph = self
            .trane
            .as_ref()
            .unwrap()
            .generate_dot_graph(courses_only);
        let mut file = File::create(path)?;
        file.write_all(dot_graph.as_bytes())?;
        Ok(())
    }

    /// Filters out any empty ID from the given list.
    fn filter_empty_ids(ids: &[Ustr]) -> Vec<Ustr> {
        ids.iter().filter(|id| !id.is_empty()).copied().collect()
    }

    /// Sets the filter to only show exercises from the given courses.
    pub fn filter_courses(&mut self, course_ids: &[Ustr]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_ids = Self::filter_empty_ids(course_ids);
        for course_id in &course_ids {
            let unit_type = self.get_unit_type(*course_id)?;
            if unit_type != UnitType::Course {
                bail!("Unit with ID {} is not a course", course_id);
            }
        }

        self.filter = Some(UnitFilter::CourseFilter { course_ids });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises from the given lessons.
    pub fn filter_lessons(&mut self, lesson_ids: &[Ustr]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_ids = Self::filter_empty_ids(lesson_ids);
        for lesson_id in &lesson_ids {
            let unit_type = self.get_unit_type(*lesson_id)?;
            if unit_type != UnitType::Lesson {
                bail!("Unit with ID {} is not a lesson", lesson_id);
            }
        }

        self.filter = Some(UnitFilter::LessonFilter { lesson_ids });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises which belong to any course or lesson with the given
    /// metadata.
    pub fn filter_metadata(
        &mut self,
        filter_op: FilterOp,
        lesson_metadata: Option<&Vec<KeyValue>>,
        course_metadata: Option<&Vec<KeyValue>>,
    ) {
        let basic_lesson_filters: Vec<_> = lesson_metadata
            .as_ref()
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| KeyValueFilter::LessonFilter {
                        key: pair.key.clone(),
                        value: pair.value.clone(),
                        filter_type: FilterType::Include,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let basic_course_filters: Vec<_> = course_metadata
            .as_ref()
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| KeyValueFilter::CourseFilter {
                        key: pair.key.clone(),
                        value: pair.value.clone(),
                        filter_type: FilterType::Include,
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.filter = Some(UnitFilter::MetadataFilter {
            filter: KeyValueFilter::CombinedFilter {
                op: filter_op,
                filters: basic_lesson_filters
                    .iter()
                    .chain(basic_course_filters.iter())
                    .cloned()
                    .collect(),
            },
        });
        self.reset_batch();
    }

    /// Sets the filter to only show exercises from the review list.
    pub fn filter_review_list(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.filter = Some(UnitFilter::ReviewListFilter);
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises starting from the dependencies of the given units at
    /// the given depth.
    pub fn filter_dependencies(&mut self, unit_ids: &[Ustr], depth: usize) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_ids = Self::filter_empty_ids(unit_ids);
        self.filter = Some(UnitFilter::Dependencies { unit_ids, depth });
        self.reset_batch();
        Ok(())
    }

    /// Sets the filter to only show exercises from the given units and their dependents.
    pub fn filter_dependents(&mut self, unit_ids: &[Ustr]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_ids = Self::filter_empty_ids(unit_ids);
        self.filter = Some(UnitFilter::Dependents { unit_ids });
        self.reset_batch();
        Ok(())
    }

    /// Returns the type of the unit with the given ID.
    pub fn get_unit_type(&self, unit_id: Ustr) -> Result<UnitType> {
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
        println!("{:<30} {:<50}", "ID", "Description");
        for filter in filters {
            println!("{:<30} {:<50}", filter.0, filter.1);
        }
        Ok(())
    }

    /// Prints the info of the given units to the terminal.
    fn print_units_info(&self, unit_ids: &[Ustr]) -> Result<()> {
        println!("{:<15} {:<50}", "Unit Type", "Unit ID");
        for unit_id in unit_ids {
            let unit_type = self.get_unit_type(*unit_id)?;
            println!("{unit_type:<15} {unit_id:<50}");
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
        self.print_units_info(&courses)?;
        Ok(())
    }

    /// Lists the dependencies of the given unit.
    pub fn list_dependencies(&self, unit_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        if unit_type == UnitType::Exercise {
            bail!("Exercises do not have dependencies");
        }

        let dependencies = self
            .trane
            .as_ref()
            .unwrap()
            .get_dependencies(unit_id)
            .unwrap_or_default();
        if dependencies.is_empty() {
            println!("No dependencies for unit with ID {unit_id}");
            return Ok(());
        }

        println!("Dependencies:");
        println!();
        self.print_units_info(&dependencies.iter().copied().collect::<Vec<_>>())?;
        Ok(())
    }

    /// Lists the dependents of the given unit.
    pub fn list_dependents(&self, unit_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        if unit_type == UnitType::Exercise {
            bail!("Exercises do not have dependents");
        }

        let dependents = self
            .trane
            .as_ref()
            .unwrap()
            .get_dependents(unit_id)
            .unwrap_or_default();
        if dependents.is_empty() {
            println!("No dependents for unit with ID {unit_id}");
            return Ok(());
        }

        println!("Dependents:");
        println!();
        self.print_units_info(&dependents.iter().copied().collect::<Vec<_>>())?;
        Ok(())
    }

    /// Lists the IDs of all the exercises in the given lesson.
    pub fn list_exercises(&self, lesson_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let exercises = self
            .trane
            .as_ref()
            .unwrap()
            .get_exercise_ids(lesson_id)
            .unwrap_or_default();
        if exercises.is_empty() {
            println!("No exercises in lesson {lesson_id}");
            return Ok(());
        }

        println!("Exercises:");
        println!();
        self.print_units_info(&exercises)?;
        Ok(())
    }

    /// Lists the IDs of all the lessons in the given course.
    pub fn list_lessons(&self, course_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lessons = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_ids(course_id)
            .unwrap_or_default();
        if lessons.is_empty() {
            println!("No lessons in course {course_id}");
            return Ok(());
        }

        println!("Lessons:");
        println!();
        self.print_units_info(&lessons)?;
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
                let manifest = self.trane.as_ref().unwrap().get_course_manifest(*course_id);
                match manifest {
                    Some(manifest) => match filter {
                        UnitFilter::CourseFilter { .. } => filter.passes_course_filter(course_id),
                        UnitFilter::LessonFilter { .. } => false,
                        UnitFilter::MetadataFilter { filter } => filter.apply_to_course(&manifest),
                        UnitFilter::Dependents { unit_ids }
                        | UnitFilter::Dependencies { unit_ids, .. } => unit_ids.contains(course_id),
                        UnitFilter::ReviewListFilter => {
                            if let Ok(review_units) =
                                self.trane.as_ref().unwrap().get_review_list_entries()
                            {
                                review_units.contains(course_id)
                            } else {
                                false
                            }
                        }
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
            println!("{course}");
        }
        Ok(())
    }

    /// Lists all the lessons in the given course which match the current filter.
    pub fn list_matching_lessons(&self, course_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lessons: Vec<Ustr> = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_ids(course_id)
            .unwrap_or_default()
            .into_iter()
            .filter(|lesson_id| {
                if self.filter.is_none() {
                    return true;
                }

                let filter = self.filter.as_ref().unwrap();
                let lesson_manifest = self.trane.as_ref().unwrap().get_lesson_manifest(*lesson_id);
                match lesson_manifest {
                    Some(lesson_manifest) => match filter {
                        UnitFilter::CourseFilter { .. } => {
                            filter.passes_course_filter(&lesson_manifest.course_id)
                        }
                        UnitFilter::LessonFilter { .. } => filter.passes_lesson_filter(lesson_id),
                        UnitFilter::MetadataFilter { filter } => {
                            let course_manifest = self
                                .trane
                                .as_ref()
                                .unwrap()
                                .get_course_manifest(lesson_manifest.course_id);
                            if course_manifest.is_none() {
                                // This should never happen but print the lesson ID if it does.
                                return true;
                            }
                            let course_manifest = course_manifest.unwrap();
                            filter.apply_to_lesson(&course_manifest, &lesson_manifest)
                        }
                        UnitFilter::ReviewListFilter => {
                            if let Ok(review_units) =
                                self.trane.as_ref().unwrap().get_review_list_entries()
                            {
                                review_units.contains(lesson_id)
                            } else {
                                false
                            }
                        }
                        UnitFilter::Dependencies { unit_ids, .. }
                        | UnitFilter::Dependents { unit_ids } => unit_ids.contains(lesson_id),
                    },
                    None => false,
                }
            })
            .collect();

        if lessons.is_empty() {
            println!("No matching lessons in course {course_id}");
            return Ok(());
        }

        println!("Lessons:");
        println!();
        for lesson in lessons {
            println!("{lesson}");
        }
        Ok(())
    }

    /// Returns the exercise filter to use, which is either a unit filter or a study session.
    fn exercise_filter(&self) -> Option<ExerciseFilter> {
        match self.filter {
            None => self
                .study_session
                .as_ref()
                .map(|study_session| ExerciseFilter::StudySession(study_session.clone())),
            Some(ref filter) => Some(ExerciseFilter::UnitFilter(filter.clone())),
        }
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
                .get_exercise_batch(self.exercise_filter())?;
            self.batch_index = 0;
        }

        let manifest = self.current_exercise()?;
        manifest.display_exercise()
    }

    /// Opens the course library at the given path.
    pub fn open_library(&mut self, library_root: &str) -> Result<()> {
        let trane = Trane::new_local(&std::env::current_dir()?, Path::new(library_root))?;
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

    /// Sets the unit filter to the saved filter with the given ID. Setting a filter resets the
    /// study session, as only one of the two can be active at a time.
    pub fn set_filter(&mut self, filter_id: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let saved_filter = self
            .trane
            .as_ref()
            .unwrap()
            .get_filter(filter_id)
            .ok_or_else(|| anyhow!("no filter with ID {}", filter_id))?;
        self.filter = Some(saved_filter.filter);
        self.study_session = None;
        self.reset_batch();
        Ok(())
    }

    /// Shows the answer to the current exercise.
    pub fn show_answer(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let curr_exercise = self.current_exercise()?;
        curr_exercise.display_answer()
    }

    /// Lists all the entries in the blacklist.
    pub fn list_blacklist(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let trane = self.trane.as_ref().unwrap();
        let entries = trane.get_blacklist_entries()?;
        if entries.is_empty() {
            println!("No entries in the blacklist");
            return Ok(());
        }

        println!("{:<15} Unit ID", "Unit Type");
        for unit_id in entries {
            let unit_type = if let Some(ut) = trane.get_unit_type(unit_id) {
                ut.to_string()
            } else {
                "Unknown".to_string()
            };
            println!("{unit_type:<15} {unit_id}");
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
    pub fn show_course_instructions(&self, course_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_id = self.course_id_or_current(course_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_course_manifest(course_id)
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
    pub fn show_lesson_instructions(&self, lesson_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.lesson_id_or_current(lesson_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_manifest(lesson_id)
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
    pub fn show_course_material(&self, course_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let course_id = self.course_id_or_current(course_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_course_manifest(course_id)
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
    pub fn show_lesson_material(&self, lesson_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let lesson_id = self.lesson_id_or_current(lesson_id)?;
        let manifest = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_manifest(lesson_id)
            .ok_or_else(|| anyhow!("no manifest for lesson with ID {}", lesson_id))?;
        match manifest.lesson_material {
            None => {
                println!("Lesson has no material");
                Ok(())
            }
            Some(material) => material.display_asset(),
        }
    }

    /// Shows the current count of Tara Sarasvati mantras. Her mantra is "recited" by the
    /// `mantra-mining` library in the background as a symbolic way in which users can contribute
    /// back to the maintainers of this program. See more information in the README of the
    /// `mantra-mining` library.
    pub fn show_mantra_count(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        println!(
            "Mantra count: {}",
            self.trane.as_ref().unwrap().mantra_count()
        );
        Ok(())
    }

    /// Shows the most recent scores for the given exercise.
    pub fn show_scores(
        &self,
        exercise_id: Ustr,
        num_scores: usize,
        num_rewards: usize,
    ) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        // Retrieve and validate the exercise, course, and lesson IDs.
        let exercise_id = self.exercise_id_or_current(exercise_id)?;
        if let Some(UnitType::Exercise) = self.trane.as_ref().unwrap().get_unit_type(exercise_id) {
        } else {
            bail!("Unit with ID {} is not a valid exercise", exercise_id);
        }
        let lesson_id = self
            .trane
            .as_ref()
            .unwrap()
            .get_exercise_lesson(exercise_id)
            .ok_or_else(|| anyhow!("no lesson for exercise with ID {}", exercise_id))?;
        let course_id = self
            .trane
            .as_ref()
            .unwrap()
            .get_lesson_course(lesson_id)
            .ok_or_else(|| anyhow!("no course for lesson with ID {}", lesson_id))?;

        // Retrieve the scores and rewards and compute the aggregate score.
        let scores = self
            .trane
            .as_ref()
            .unwrap()
            .get_scores(exercise_id, num_scores)?;
        let lesson_rewards = self
            .trane
            .as_ref()
            .unwrap()
            .get_rewards(lesson_id, num_rewards)
            .unwrap_or_default();
        let course_rewards = self
            .trane
            .as_ref()
            .unwrap()
            .get_rewards(course_id, num_rewards)
            .unwrap_or_default();

        let decay_scorer = ExponentialDecayScorer {};
        let reward_scorer = WeightedRewardScorer {};
        let score = decay_scorer.score(&scores)?;
        let reward = reward_scorer.score_rewards(&course_rewards, &lesson_rewards)?;
        let total_score = if score > 0.0 {
            (score + reward).clamp(0.0, 5.0)
        } else {
            0.0
        };

        // Print the scores.
        println!("Scores for exercise {exercise_id}:");
        println!();
        println!("Note: Rewards are only applied to exercises with previous scores");
        println!();
        println!("Score: {score:.2}");
        println!("Reward: {reward:.2}");
        println!("Final score: {total_score:.2}");
        println!();
        println!("Raw scores:");
        println!("{:<25} {:>6}", "Date", "Score");
        for score in scores {
            if let Some(dt) = Local.timestamp_opt(score.timestamp, 0).earliest() {
                println!(
                    "{:<25} {:>6}",
                    dt.format("%Y-%m-%d %H:%M:%S"),
                    score.score as u8
                );
            }
        }
        Ok(())
    }

    /// Prints the manifest for the unit with the given UID.
    fn show_unit_manifest(&self, unit_id: Ustr, unit_type: &UnitType) -> Result<()> {
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
                println!("{manifest:#?}");
            }
            UnitType::Lesson => {
                let manifest = self
                    .trane
                    .as_ref()
                    .unwrap()
                    .get_lesson_manifest(unit_id)
                    .ok_or_else(|| anyhow!("missing manifest for lesson {}", unit_id))?;
                println!("Unit manifest:");
                println!("{manifest:#?}");
            }
            UnitType::Course => {
                let manifest = self
                    .trane
                    .as_ref()
                    .unwrap()
                    .get_course_manifest(unit_id)
                    .ok_or_else(|| anyhow!("missing manifest for course {}", unit_id))?;
                println!("Unit manifest:");
                println!("{manifest:#?}");
            }
        }
        Ok(())
    }

    /// Prints information about the given unit.
    pub fn show_unit_info(&self, unit_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let unit_type = self.get_unit_type(unit_id)?;
        println!("Unit ID: {unit_id}");
        println!("Unit Type: {unit_type}");
        self.show_unit_manifest(unit_id, &unit_type)
    }

    /// Trims the scores for each exercise by removing all the scores except for the `num_scores`
    /// most recent scores.
    pub fn trim_scores(&mut self, num_scores: usize) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().trim_scores(num_scores)?;
        println!("Trimmed scores for all exercises");
        Ok(())
    }

    /// Removes the scores for exercises that match the given prefix.
    pub fn remove_prefix_from_scores(&mut self, prefix: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane
            .as_mut()
            .unwrap()
            .remove_scores_with_prefix(prefix)?;
        println!("Removed scores for all exercises with prefix {prefix}");
        Ok(())
    }

    /// Removes the given unit from the blacklist.
    pub fn remove_from_blacklist(&mut self, unit_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_mut()
            .unwrap()
            .remove_from_blacklist(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Removes the given unit from the blacklist.
    pub fn remove_prefix_from_blacklist(&mut self, prefix: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_mut()
            .unwrap()
            .remove_prefix_from_blacklist(prefix)?;
        self.reset_batch();
        Ok(())
    }

    /// Adds a new repository to the Trane instance.
    pub fn add_repo(&mut self, url: &str, repo_id: Option<String>) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().add_repo(url, repo_id)?;
        Ok(())
    }

    /// Removes the given repository from the Trane instance.
    pub fn remove_repo(&mut self, repo_id: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().remove_repo(repo_id)?;
        Ok(())
    }

    /// Lists all the repositories managed by the Trane instance.
    pub fn list_repos(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        let repos = self.trane.as_ref().unwrap().list_repos();
        if repos.is_empty() {
            println!("No repositories are managed by Trane");
            return Ok(());
        }

        println!("{:<20} URL", "ID");
        for repo in repos {
            println!("{:<20} {}", repo.id, repo.url);
        }
        Ok(())
    }

    /// Updates the given repository.
    pub fn update_repo(&mut self, repo_id: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().update_repo(repo_id)?;
        Ok(())
    }

    /// Updates all the repositories managed by the Trane instance.
    pub fn update_all_repos(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().update_all_repos()?;
        Ok(())
    }

    /// Adds the given unit to the review list.
    pub fn add_to_review_list(&mut self, unit_id: Ustr) -> Result<()> {
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
    pub fn remove_from_review_list(&mut self, unit_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        self.trane
            .as_mut()
            .unwrap()
            .remove_from_review_list(unit_id)?;
        self.reset_batch();
        Ok(())
    }

    /// Lists all the units in the review list.
    pub fn list_review_list(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let entries = self.trane.as_ref().unwrap().get_review_list_entries()?;
        if entries.is_empty() {
            println!("No entries in the blacklist");
            return Ok(());
        }

        println!("Review list:");
        println!("{:<10} {:<50}", "Unit Type", "Unit ID");
        for unit_id in entries {
            let unit_type = self.get_unit_type(unit_id);
            if unit_type.is_err() {
                println!("{:<10} {:<50}", "Unknown", unit_id.as_str());
            } else {
                println!("{:<10} {:<50}", unit_type.unwrap(), unit_id.as_str());
            }
        }
        Ok(())
    }

    /// Searches for units which match the given query.
    pub fn search(&self, terms: &[String]) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        ensure!(!terms.is_empty(), "no search terms given");

        let query = terms
            .iter()
            .map(|s| {
                let mut quoted = "\"".to_string();
                quoted.push_str(s);
                quoted.push('"');
                quoted
            })
            .collect::<Vec<_>>()
            .join(" ");
        let results = self.trane.as_ref().unwrap().search(&query)?;

        if results.is_empty() {
            println!("No results found");
            return Ok(());
        }

        println!("Search results:");
        println!("{:<10} {:<50}", "Unit Type", "Unit ID");
        for unit_id in results {
            let unit_type = self.get_unit_type(unit_id)?;
            println!("{unit_type:<10} {unit_id:<50}");
        }
        Ok(())
    }

    /// Resets the scheduler options to their default values.
    pub fn reset_scheduler_options(&mut self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().reset_scheduler_options();
        Ok(())
    }

    /// Sets the scheduler options.
    pub fn set_scheduler_options(&mut self, options: SchedulerOptions) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        self.trane.as_mut().unwrap().set_scheduler_options(options);
        Ok(())
    }

    /// Shows the current scheduler options.
    pub fn show_scheduler_options(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");
        let options = self.trane.as_ref().unwrap().get_scheduler_options();
        println!("{options:#?}");
        Ok(())
    }

    /// Clears the study session if it's set.
    pub fn clear_study_session(&mut self) {
        if self.filter.is_none() {
            return;
        }
        self.filter = None;
        self.study_session = None;
        self.reset_batch();
    }

    /// Prints the list of all the saved unit filters.
    pub fn list_study_sessions(&self) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let sessions = self.trane.as_ref().unwrap().list_study_sessions();
        if sessions.is_empty() {
            println!("No saved study sessions");
            return Ok(());
        }

        println!("Saved study sessions:");
        println!("{:<30} {:<50}", "ID", "Description");
        for filter in sessions {
            println!("{:<30} {:<50}", filter.0, filter.1);
        }
        Ok(())
    }

    /// Sets the study session to the saved session with the given ID. Setting a study session
    /// resets the filter, as only one of the two can be active at a time.
    pub fn set_study_session(&mut self, session_id: &str) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let saved_session = self
            .trane
            .as_ref()
            .unwrap()
            .get_study_session(session_id)
            .ok_or_else(|| anyhow!("no study session with ID {}", session_id))?;
        self.filter = None;
        self.study_session = Some(StudySessionData {
            start_time: Utc::now(),
            definition: saved_session,
        });
        self.reset_batch();
        Ok(())
    }

    /// Shows the currently set study session.
    pub fn show_study_session(&self) {
        if self.filter.is_none() {
            println!("No study session is set");
        } else {
            println!("Study session:");
            println!("{:#?}", self.study_session.as_ref().unwrap());
        }
    }

    /// Prints the path to the transcription asset for the given exercise.
    pub fn transcription_path(&self, exercise_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let trane = self.trane.as_ref().unwrap();
        let path = trane.transcription_download_path(exercise_id);
        if let Some(path) = path {
            println!("Transcription asset download path: {}", path.display());
        }
        let alias_path = trane.transcription_download_path_alias(exercise_id);
        if let Some(alias_path) = alias_path {
            println!(
                "Transcription asset download path alias: {}",
                alias_path.display()
            );
        }
        Ok(())
    }

    /// Downloads the transcription asset from the given exercise to the specified directory in the
    /// user preferences.
    pub fn download_transcription_asset(&self, exercise_id: Ustr, redownload: bool) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let exercise_id = self.exercise_id_or_current(exercise_id)?;
        self.trane
            .as_ref()
            .unwrap()
            .download_transcription_asset(exercise_id, redownload)?;
        println!("Transcription asset for exercise {exercise_id} downloaded");
        println!();
        self.transcription_path(exercise_id)?;
        Ok(())
    }

    /// Prints whether the transcription asset for the given exercise has been downloaded.
    pub fn is_transcription_asset_downloaded(&self, exercise_id: Ustr) -> Result<()> {
        ensure!(self.trane.is_some(), "no Trane instance is open");

        let exercise_id = self.exercise_id_or_current(exercise_id)?;
        let trane = self.trane.as_ref().unwrap();
        let is_downloaded = trane.is_transcription_asset_downloaded(exercise_id);
        if is_downloaded {
            println!("Transcription for exercise {exercise_id} is downloaded");
            println!();
            self.transcription_path(exercise_id)?;
        } else {
            println!("Transcription for exercise {exercise_id} is not downloaded");
        }
        Ok(())
    }
}
