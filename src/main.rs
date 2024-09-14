#![forbid(unsafe_code)]
#![forbid(clippy::float_arithmetic)]
#![forbid(future_incompatible)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![warn(clippy::cargo)]
#![deny(missing_debug_implementations)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(dead_code)]
#![deny(unreachable_code)]
#![deny(unused_mut)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unexpected_cfgs,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    nonstandard_style,
    rust_2018_idioms
)]
use std::collections::HashMap;

use crate::colors::Tend;
mod args;
mod colors;
mod job;
mod run;

use crate::job::{filter::Filter, Job};
use anyhow::Result;
use clap::Parser;

fn standard_job_filter(
    name: Option<String>,
    _all: bool,
    group: Vec<String>,
    job: Vec<String>,
    exclude: Vec<String>,
) -> Filter {
    if group.is_empty() && job.is_empty() {
        if let Some(name) = name {
            Filter::Subset {
                groups: vec![],
                jobs: vec![name],
                exclude,
            }
        } else {
            Filter::All { exclude }
        }
    } else {
        Filter::Subset {
            groups: group,
            jobs: job,
            exclude,
        }
    }
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let args = args::Cli::parse();

    if args.no_color {
        colored::control::set_override(false);
    }

    match args.command {
        args::Commands::List {
            all,
            group,
            job,
            exclude,
            name,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            match Job::list(&filter) {
                Ok(()) => (),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        args::Commands::Run {
            name,
            group,
            job,
            all,
            exclude,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            run::run(filter, args.verbose).await?;
        }
        args::Commands::Create {
            name,
            program,
            args,
            restart,
            group,
            overwrite,
            restart_strategy,
            template,
        } => {
            let mut job = Job {
                name,
                program,
                args,
                restart,
                group,
                working_directory: std::env::current_dir()?,
                restart_strategy,
                event_hooks: HashMap::new(),
                template,
            };

            if let Some(template) = template {
                job.apply_template(template);
            }

            let res = job.save(overwrite);
            if let Err(ref error) = res {
                if let Some(error) = error.downcast_ref::<std::io::Error>() {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        eprintln!(
                            "{}",
                            "Job already exists. Use --overwrite to replace it.".failure()
                        );
                        return Ok(());
                    }
                }
            }
            res?;
        }
        args::Commands::Edit { name, command } => {
            let mut job =
                Job::load(&name).ok_or_else(|| anyhow::anyhow!("Job could not be loaded."))?;
            match command {
                args::EditJobCommands::Group { group } => job.group = group,
                args::EditJobCommands::Hook { command } => match command {
                    args::EditJobHookCommands::List => {
                        if job.event_hooks.is_empty() {
                            println!("No hooks defined for job {}", job.name);
                        } else {
                            for (name, hook) in &job.event_hooks {
                                println!("{name}: {hook:?}");
                            }
                        }
                    }
                    args::EditJobHookCommands::Create { hook, t } => match t {
                        args::JobHook::DetectSubstring {
                            substring,
                            stream,
                            action,
                        } => {
                            job.event_hooks.insert(
                                hook,
                                job::event::Hook {
                                    event: job::event::Event::DetectSubstring {
                                        contains: substring,
                                        stream,
                                    },
                                    action,
                                },
                            );
                        }
                    },
                    args::EditJobHookCommands::Delete { hook } => {
                        match job.event_hooks.remove(&hook) {
                            Some(_) => println!("Hook {hook} deleted"),
                            None => eprintln!("Hook {hook} not found"),
                        }
                    }
                },
            }
            job.save(true)?;
        }
        args::Commands::Delete {
            name,
            group,
            all,
            confirm,
            job,
            exclude,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            if all && !confirm {
                eprintln!(
                    "{}",
                    "Use --confirm to delete all jobs. This cannot be undone.".failure()
                );
            } else {
                Job::iterate_jobs_filtered(
                    |job| {
                        let _ = job.delete();
                    },
                    &filter,
                )?;
            }
        }
    }

    Ok(())
}
