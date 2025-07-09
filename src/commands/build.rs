use crate::api::APIClient;
use crate::commands::CommandBase;
use anyhow::{anyhow, Result};
use bollard::models::{BuildInfo, BuildInfoAux};
use bollard::moby::buildkit::v1::StatusResponse;
use bollard::query_parameters::{ListImagesOptions, BuildImageOptions, BuilderVersion};
use bollard::Docker;
use clap::Parser;
use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Constants
const DEFAULT_PLATFORM: &str = "linux/amd64";
const SHA_DISPLAY_LENGTH: usize = 8;

#[derive(Debug, Clone)]
struct BuildEvent {
    name: String,
    step_number: Option<u32>,
    started: bool,
    completed: bool,
    cached: bool,
    logs: Vec<String>,
}

// Custom logger that shows progress instead of raw output
struct ProgressLogger {
    progress_bar: ProgressBar,
    show_raw_output: bool,
    build_stats: std::sync::Mutex<BuildStats>,
}

#[derive(Debug, Default)]
struct BuildStats {
    total_steps: u32,
    current_step: u32,
    cache_hits: u32,
    cache_misses: u32,
    layers_processed: u32,
    total_image_size: u64,
    layers_downloaded: u32,
    build_start_time: Option<Instant>,
    vertex_states: HashMap<String, bool>, // digest -> final cached state
    base_image_layers: u32, // layers from base image that were already present
    build_log: Vec<String>, // full build log for error reporting
    build_events: std::collections::HashMap<String, BuildEvent>, // deduplicated build events by vertex digest
}

impl ProgressLogger {
    fn new(show_raw_output: bool) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("🏗️  Initializing build...");

        Self {
            progress_bar: pb,
            show_raw_output,
            build_stats: std::sync::Mutex::new(BuildStats::default()),
        }
    }

    fn update_progress(&self, message: &str) {
        // Parse common Docker build steps and show meaningful progress
        if message.contains("COPY") {
            self.progress_bar.set_message("📄 Copying build context...");
        } else if message.contains("RUN") {
            self.progress_bar
                .set_message("⚙️  Executing build steps...");
        } else if message.contains("FROM") {
            self.progress_bar.set_message("📦 Pulling base image...");
        } else if message.contains("WORKDIR") {
            self.progress_bar
                .set_message("📁 Setting up working directory...");
        } else if message.contains("EXPOSE") {
            self.progress_bar.set_message("🔌 Configuring ports...");
        } else if message.contains("CMD") || message.contains("ENTRYPOINT") {
            self.progress_bar.set_message("🎯 Setting up entrypoint...");
        } else if message.contains("export") {
            self.progress_bar.set_message("💾 Exporting image...");
        } else if message.contains("resolve") {
            self.progress_bar
                .set_message("🔍 Resolving dependencies...");
        } else if message.contains("build") {
            self.progress_bar.set_message("🏗️  Building image...");
        } else if !message.trim().is_empty() {
            // Generic progress message for non-empty logs
            self.progress_bar
                .set_message(format!("🔄 {}", message.trim()));
        }
        self.progress_bar.tick();
    }

    fn finish(&self, message: &str) {
        let summary = self.get_build_summary();
        let full_message = if !summary.is_empty() {
            format!("{}\n{}", message, summary)
        } else {
            message.to_string()
        };
        self.progress_bar.finish_with_message(full_message);
    }

    fn set_message(&self, message: &str) {
        self.progress_bar.set_message(message.to_string());
        self.progress_bar.tick();
    }

    fn handle_build_output(&self, output: &str) {
        // Always store the output in build log for error reporting
        {
            let mut stats = self.build_stats.lock().unwrap();
            stats.build_log.push(output.to_string());
        }
        
        if self.show_raw_output {
            println!("{}", output);
        } else {
            self.update_progress(output);
        }
    }

    fn handle_build_error(&self, error: &str) {
        // Store the error in build log
        {
            let mut stats = self.build_stats.lock().unwrap();
            stats.build_log.push(format!("ERROR: {}", error));
        }
        
        if self.show_raw_output {
            eprintln!("{}", error);
        } else {
            self.update_progress(error);
        }
        
        // Print full build log that led up to this error
        self.print_full_build_log_on_error();
    }

    fn parse_and_display_build_output(&self, output: &str) {
        // Parse Docker/BuildKit output for meaningful progress updates
        let trimmed = output.trim();
        
        if trimmed.starts_with("STEP") {
            // Extract step information: "STEP 1/4: FROM alpine:latest"
            if let Some(step_info) = parse_step_info(trimmed) {
                self.update_step_stats(&step_info);
                let stats = self.build_stats.lock().unwrap();
                let cache_ratio = if stats.cache_hits + stats.cache_misses > 0 {
                    format!(" • Cache: {:.1}%", 
                        (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0)
                } else {
                    String::new()
                };
                self.progress_bar.set_message(format!("🏗️  Step {}/{}: {}{}", 
                    step_info.current, step_info.total, step_info.instruction, cache_ratio));
            }
        } else if trimmed.starts_with("Trying to pull") {
            // "Trying to pull docker.io/library/alpine:latest..."
            if let Some(image) = extract_image_name(trimmed) {
                let stats = self.build_stats.lock().unwrap();
                self.progress_bar.set_message(format!("📦 Pulling {} • Layer {}", 
                    image, stats.layers_processed + 1));
            }
        } else if trimmed.starts_with("Getting image source signatures") {
            self.progress_bar.set_message("🔐 Verifying image signatures...");
        } else if trimmed.starts_with("Copying blob") {
            // Extract blob info: "Copying blob sha256:fe07684b16b82247c3539ed86a65ff37a76138ec25d380bd80c869a1a4c73236"
            if let Some(blob_info) = extract_blob_info(trimmed) {
                self.update_download_stats(&blob_info);
                let stats = self.build_stats.lock().unwrap();
                self.progress_bar.set_message(format!("📥 Downloading layer {} • {} layers total", 
                    &blob_info.id[..8], stats.layers_downloaded));
            } else {
                self.progress_bar.set_message("📥 Downloading layers...");
            }
        } else if trimmed.starts_with("Copying config") {
            self.progress_bar.set_message("⚙️  Copying configuration...");
        } else if trimmed.starts_with("Writing manifest") {
            self.progress_bar.set_message("📝 Writing manifest...");
        } else if trimmed.starts_with("-->") {
            // Layer completion: "--> e63fd7e7b356" or cache hit: "--> Using cache 0dca35029b5a"
            if trimmed.contains("Using cache") {
                let cache_part = trimmed.trim_start_matches("-->").trim();
                if let Some(layer_id) = cache_part.strip_prefix("Using cache ") {
                    self.update_cache_stats(layer_id, true);
                    let stats = self.build_stats.lock().unwrap();
                    self.progress_bar.set_message(format!("♻️  Cached layer {} • {}/{} cached", 
                        &layer_id[..8], stats.cache_hits, stats.cache_hits + stats.cache_misses));
                } else {
                    self.update_cache_stats("unknown", true);
                    self.progress_bar.set_message("♻️  Using cached layer");
                }
            } else {
                let layer_id = trimmed.trim_start_matches("-->").trim();
                self.update_cache_stats(layer_id, false);
                let stats = self.build_stats.lock().unwrap();
                self.progress_bar.set_message(format!("✅ Layer {} built • {}/{} from cache", 
                    &layer_id[..8], stats.cache_hits, stats.cache_hits + stats.cache_misses));
            }
        } else if trimmed.starts_with("COMMIT") {
            // "COMMIT docker.io/library/test-build:latest"
            let stats = self.build_stats.lock().unwrap();
            self.progress_bar.set_message(format!("💾 Committing image • {} layers", stats.layers_processed));
        } else if trimmed.starts_with("Successfully tagged") {
            // "Successfully tagged docker.io/library/test-build:latest"
            if let Some(tag) = extract_tag_name(trimmed) {
                self.progress_bar.set_message(format!("🏷️  Tagged as {}", tag));
            }
        } else if trimmed.starts_with("Successfully built") {
            // "Successfully built 59c90a041ff7"
            let build_id = trimmed.trim_start_matches("Successfully built").trim();
            let stats = self.build_stats.lock().unwrap();
            let cache_ratio = if stats.cache_hits + stats.cache_misses > 0 {
                (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0
            } else {
                0.0
            };
            self.progress_bar.set_message(format!("🎉 Build completed! ID: {} • {:.1}% cached", 
                &build_id[..8], cache_ratio));
        } else if trimmed.contains("RUN") {
            self.progress_bar.set_message("⚙️  Executing commands...");
        } else if trimmed.contains("COPY") {
            self.progress_bar.set_message("📄 Copying files...");
        } else if trimmed.contains("FROM") {
            self.progress_bar.set_message("🏗️  Setting up base image...");
        } else if trimmed.contains("WORKDIR") {
            self.progress_bar.set_message("📁 Setting working directory...");
        } else if trimmed.contains("EXPOSE") {
            self.progress_bar.set_message("🔌 Configuring ports...");
        } else if trimmed.contains("CMD") || trimmed.contains("ENTRYPOINT") {
            self.progress_bar.set_message("🎯 Setting up entrypoint...");
        } else if !trimmed.is_empty() {
            // Generic progress message for any other output
            self.progress_bar.set_message(format!("🔄 {}", trimmed));
        }
        
        self.progress_bar.tick();
    }

    fn update_step_stats(&self, step_info: &StepInfo) {
        let mut stats = self.build_stats.lock().unwrap();
        stats.total_steps = step_info.total;
        stats.current_step = step_info.current;
        if stats.build_start_time.is_none() {
            stats.build_start_time = Some(Instant::now());
        }
    }

    fn update_cache_stats(&self, _layer_id: &str, is_cache_hit: bool) {
        let mut stats = self.build_stats.lock().unwrap();
        stats.layers_processed += 1;
        if is_cache_hit {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
        }
    }

    fn update_download_stats(&self, blob_info: &BlobInfo) {
        let mut stats = self.build_stats.lock().unwrap();
        stats.layers_downloaded += 1;
        stats.total_image_size += blob_info.size;
    }

    fn print_full_build_log_on_error(&self) {
        let stats = self.build_stats.lock().unwrap();
        
        eprintln!("\n🚫 ========== BUILD FAILURE DETAILS ==========\n");
        eprintln!("📋 Full build log leading up to the error:\n");
        
        // Collect and sort build events: internal events first, then by step number
        let mut ordered_events: Vec<_> = stats.build_events.values()
            .filter(|event| {
                // Only show events that actually started or have logs
                event.started || !event.logs.is_empty() || event.name.contains("[internal]")
            })
            .collect();
        
        ordered_events.sort_by(|a, b| {
            let a_is_internal = a.name.contains("[internal]");
            let b_is_internal = b.name.contains("[internal]");
            
            match (a_is_internal, b_is_internal) {
                // Both internal - sort by name
                (true, true) => a.name.cmp(&b.name),
                // Internal events come first
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                // Both are Dockerfile steps - sort by step number
                (false, false) => {
                    match (a.step_number, b.step_number) {
                        (Some(a_num), Some(b_num)) => a_num.cmp(&b_num),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.cmp(&b.name),
                    }
                }
            }
        });
        
        let mut line_number = 1;
        
        // Display deduplicated build events in proper order
        for event in &ordered_events {
            if !event.name.trim().is_empty() {
                // Show the main step
                if event.name.starts_with("[") {
                    let status_icon = if event.cached {
                        "♻️"
                    } else if event.completed {
                        "🔹"
                    } else if event.started {
                        "⚙️"
                    } else {
                        "⏳"
                    };
                    eprintln!("{} {:3}: {}", status_icon, line_number, event.name);
                    line_number += 1;
                } else if event.name.contains("[internal]") {
                    eprintln!("🔧 {:3}: {}", line_number, event.name);
                    line_number += 1;
                }
                
                // Show associated logs for this step
                for log in &event.logs {
                    if !log.trim().is_empty() {
                        eprintln!("📝 {:3}: {}", line_number, log);
                        line_number += 1;
                    }
                }
            }
        }
        
        // Add any regular build logs that weren't captured as BuildKit events
        let regular_logs: Vec<_> = stats.build_log.iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("ERROR:") || 
                (trimmed.starts_with("STEP ") && !trimmed.contains("[")) ||
                (!trimmed.starts_with("STEP:") && !trimmed.starts_with("LOG:") && 
                 !trimmed.starts_with("PROGRESS:") && !trimmed.starts_with("IMAGE_ID:"))
            })
            .collect();
        
        for line in regular_logs {
            let trimmed_line = line.trim();
            
            if trimmed_line.starts_with("ERROR:") {
                eprintln!("❌ {:3}: {}", line_number, &trimmed_line[7..]);
            } else if trimmed_line.starts_with("STEP ") {
                eprintln!("🔹 {:3}: {}", line_number, trimmed_line);
            } else if trimmed_line.contains("RUN ") {
                eprintln!("⚙️  {:3}: {}", line_number, trimmed_line);
            } else if trimmed_line.contains("COPY ") || trimmed_line.contains("ADD ") {
                eprintln!("📁 {:3}: {}", line_number, trimmed_line);
            } else if trimmed_line.contains("FROM ") {
                eprintln!("🏗️  {:3}: {}", line_number, trimmed_line);
            } else if trimmed_line.starts_with("IMAGE_ID:") {
                eprintln!("🎯 {:3}: Final image {}", line_number, &trimmed_line[10..]);
            } else if !trimmed_line.is_empty() {
                eprintln!("   {:3}: {}", line_number, trimmed_line);
            }
            line_number += 1;
        }
        
        eprintln!("\n🔍 Build context summary:");
        if stats.total_steps > 0 {
            eprintln!("   • Step {}/{} when failure occurred", stats.current_step, stats.total_steps);
        }
        if stats.layers_processed > 0 {
            eprintln!("   • {} layers processed before failure", stats.layers_processed);
        }
        if let Some(start_time) = stats.build_start_time {
            let duration = start_time.elapsed();
            eprintln!("   • Build ran for {:.1}s before failing", duration.as_secs_f64());
        }
        
        eprintln!("\n💡 Tips for debugging:");
        eprintln!("   • Check Dockerfile syntax and commands");
        eprintln!("   • Verify all COPY/ADD source files exist");
        eprintln!("   • Ensure base image is accessible");
        eprintln!("   • Run with --verbose for more detailed output");
        eprintln!("\n🚫 ==========================================\n");
    }

    fn get_build_summary(&self) -> String {
        let stats = self.build_stats.lock().unwrap();
        let total_layers = stats.cache_hits + stats.cache_misses;
        let cache_percentage = if total_layers > 0 {
            (stats.cache_hits as f64 / total_layers as f64) * 100.0
        } else {
            0.0
        };
        
        let mut lines = Vec::new();
        
        // Build a buildx-style summary
        let steps_info = if stats.total_steps > 0 {
            format!("🏗️  {} Dockerfile steps", stats.total_steps)
        } else {
            "🏗️  Build steps".to_string()
        };
        
        let cache_info = if total_layers > 0 {
            let cached_count = stats.cache_hits;
            let built_count = stats.cache_misses;
            let base_layers = stats.base_image_layers;
            
            if base_layers > 0 {
                format!("♻️  {} cached ({} base) • 🔨 {} built", cached_count, base_layers, built_count)
            } else {
                format!("♻️  {} cached • 🔨 {} built", cached_count, built_count)
            }
        } else {
            "🔨 Building layers".to_string()
        };
        
        let size_info = if stats.total_image_size > 0 {
            format!("📦 {} final image", format_bytes(stats.total_image_size))
        } else {
            "📦 Image ready".to_string()
        };
        
        let timing_info = if let Some(start_time) = stats.build_start_time {
            let duration = start_time.elapsed();
            format!("⏱️  {:.1}s total build time", duration.as_secs_f64())
        } else {
            "⏱️  Build complete".to_string()
        };
        
        // Create a nice multi-line summary
        lines.push(format!("📊 Build Statistics:"));
        lines.push(format!("   {} • {} layers total", steps_info, total_layers));
        lines.push(format!("   {} • {:.1}% cache hit rate", cache_info, cache_percentage));
        lines.push(format!("   {} • {}", size_info, timing_info));
        
        lines.join("\n")
    }

    fn handle_buildkit_event(&self, event: &BuildInfoAux) {
        // Extract and deduplicate BuildKit events for better error reporting
        {
            let mut stats = self.build_stats.lock().unwrap();
            
            match event {
                BuildInfoAux::BuildKit(status_response) => {
                    // Process vertices to track build steps
                    for vertex in &status_response.vertexes {
                        let digest = &vertex.digest;
                        let name = &vertex.name;
                        let started = vertex.started.is_some();
                        let completed = vertex.completed.is_some();
                        let cached = vertex.cached;
                        
                        // Extract step number from name like "[1/6] FROM alpine:latest"
                        let step_number = if name.starts_with("[") {
                            name.split(']').next()
                                .and_then(|s| s.trim_start_matches('[').split('/').next())
                                .and_then(|s| s.parse::<u32>().ok())
                        } else {
                            None
                        };
                        
                        // Update or create build event
                        let build_event = stats.build_events.entry(digest.clone()).or_insert_with(|| BuildEvent {
                            name: name.clone(),
                            step_number,
                            started: false,
                            completed: false,
                            cached: false,
                            logs: Vec::new(),
                        });
                        
                        // Update event status
                        if started && !build_event.started {
                            build_event.started = true;
                        }
                        if completed && !build_event.completed {
                            build_event.completed = true;
                            build_event.cached = cached;
                        }
                    }
                    
                    // Process logs and associate them with vertices
                    for log in &status_response.logs {
                        let vertex_digest = &log.vertex;
                        let log_text = String::from_utf8_lossy(&log.msg).trim().to_string();
                        if !log_text.is_empty() {
                            if let Some(build_event) = stats.build_events.get_mut(vertex_digest) {
                                if !build_event.logs.contains(&log_text) {
                                    build_event.logs.push(log_text);
                                }
                            }
                        }
                    }
                }
                BuildInfoAux::Default(image_id) => {
                    if let Some(id) = &image_id.id {
                        stats.build_log.push(format!("IMAGE_ID: {}", &id[..16.min(id.len())]));
                    }
                }
            }
        }
        
        match event {
            BuildInfoAux::BuildKit(status_response) => {
                self.parse_buildkit_status_response_direct(status_response);
            }
            BuildInfoAux::Default(image_id) => {
                if let Some(id) = &image_id.id {
                    self.progress_bar.set_message(format!("🎯 Final image: {}", &id[7..15])); // Skip "sha256:"
                }
            }
        }
        self.progress_bar.tick();
    }

    fn parse_buildkit_status_response_direct(&self, status_response: &StatusResponse) {
        // Parse the BuildKit StatusResponse struct directly
        for vertex in &status_response.vertexes {
            let name = &vertex.name;
            let cached = vertex.cached;
            let started = vertex.started.is_some();
            let completed = vertex.completed.is_some();
            let digest = &vertex.digest;
            
            // Update vertex state tracking by digest
            if completed {
                let mut stats = self.build_stats.lock().unwrap();
                
                // Check if this is the first time we're seeing this digest as completed
                let was_previously_tracked = stats.vertex_states.contains_key(digest);
                let previous_cached_state = stats.vertex_states.get(digest).copied().unwrap_or(false);
                
                // Update the vertex state
                stats.vertex_states.insert(digest.clone(), cached);
                
                // Update statistics based on the change
                if !was_previously_tracked {
                    // First time seeing this completed vertex
                    if cached {
                        stats.cache_hits += 1;
                    } else {
                        stats.cache_misses += 1;
                    }
                    stats.layers_processed += 1;
                } else if previous_cached_state != cached {
                    // The cached state changed for this vertex
                    if cached && !previous_cached_state {
                        // Changed from not cached to cached
                        stats.cache_hits += 1;
                        stats.cache_misses -= 1;
                    } else if !cached && previous_cached_state {
                        // Changed from cached to not cached
                        stats.cache_hits -= 1;
                        stats.cache_misses += 1;
                    }
                }
                
                drop(stats);
            }
            
            if name.starts_with("[") && name.contains("]") {
                // This is a Dockerfile step like "[1/3] FROM docker.io/library/alpine:latest"
                self.handle_dockerfile_step(name, cached, started, completed);
                // Note: Removed sleep as it can block async operations
            } else if name.contains("load") {
                self.progress_bar.set_message(format!("📥 {}", name));
            } else if name.contains("export") {
                self.progress_bar.set_message(format!("📦 {}", name));
            } else if name.contains("metadata") {
                self.progress_bar.set_message("🔍 Resolving image metadata...");
            } else if !name.starts_with("[internal]") && !name.is_empty() {
                self.progress_bar.set_message(format!("🔧 {}", name));
            }
        }

        for status in &status_response.statuses {
            let name = &status.name;
            if name.contains("exporting") {
                self.progress_bar.set_message("📦 Exporting layers...");
            } else if name.contains("writing") {
                self.progress_bar.set_message("💾 Writing image...");
            } else if name.contains("naming") {
                self.progress_bar.set_message("🏷️  Tagging image...");
            }
        }

        for log in &status_response.logs {
            if let Ok(msg) = String::from_utf8(log.msg.clone()) {
                // Process command output from RUN steps
                if !msg.trim().is_empty() {
                    self.progress_bar.set_message(format!("⚙️  Executing: {}", msg.trim()));
                }
            }
        }
    }

    fn handle_dockerfile_step(&self, name: &str, cached: bool, _started: bool, completed: bool) {
        // Extract step info from "[1/3] FROM docker.io/library/alpine:latest"
        if let Some(bracket_end) = name.find(']') {
            let step_part = &name[1..bracket_end];
            let instruction = &name[bracket_end + 2..]; // Skip "] "
            
            let mut stats = self.build_stats.lock().unwrap();
            
            if let Some(slash_pos) = step_part.find('/') {
                let current: u32 = step_part[..slash_pos].parse().unwrap_or(0);
                let total: u32 = step_part[slash_pos + 1..].parse().unwrap_or(0);
                
                stats.current_step = current;
                stats.total_steps = total;
            }
            
            // Statistics are now tracked at the vertex level in parse_buildkit_status_response_direct
            // This function just handles display and step tracking
            
            let cache_info = if cached {
                "♻️  "
            } else if completed {
                "✅ "
            } else {
                "🏗️  "
            };
            
            let cache_ratio = if stats.cache_hits + stats.cache_misses > 0 {
                format!(" • {:.1}% cached", 
                    (stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64) * 100.0)
            } else {
                String::new()
            };
            
            // Always show the progress message for better UX
            let progress_msg = format!("{}{}/{}: {}{}", 
                cache_info, 
                stats.current_step, 
                stats.total_steps, 
                instruction,
                cache_ratio
            );
            
            drop(stats); // Release the lock before calling set_message
            self.progress_bar.set_message(progress_msg);
        }
    }

}

#[derive(Debug)]
struct StepInfo {
    current: u32,
    total: u32,
    instruction: String,
}

fn parse_step_info(step_line: &str) -> Option<StepInfo> {
    // Parse "STEP 1/4: FROM alpine:latest"
    if let Some(colon_pos) = step_line.find(':') {
        let step_part = &step_line[5..colon_pos]; // Skip "STEP "
        let instruction = step_line[colon_pos + 1..].trim();
        
        if let Some(slash_pos) = step_part.find('/') {
            let current = step_part[..slash_pos].trim().parse().ok()?;
            let total = step_part[slash_pos + 1..].trim().parse().ok()?;
            
            return Some(StepInfo {
                current,
                total,
                instruction: instruction.to_string(),
            });
        }
    }
    None
}

fn extract_image_name(pull_line: &str) -> Option<String> {
    // Extract from "Trying to pull docker.io/library/alpine:latest..."
    if let Some(start) = pull_line.find("pull ") {
        let rest = &pull_line[start + 5..];
        if let Some(end) = rest.find("...") {
            return Some(rest[..end].to_string());
        }
        return Some(rest.to_string());
    }
    None
}

fn extract_tag_name(tagged_line: &str) -> Option<String> {
    // Extract from "Successfully tagged docker.io/library/test-build:latest"
    if let Some(start) = tagged_line.find("tagged ") {
        return Some(tagged_line[start + 7..].to_string());
    }
    None
}

#[derive(Debug)]
struct BlobInfo {
    id: String,
    size: u64,
}

fn extract_blob_info(blob_line: &str) -> Option<BlobInfo> {
    // Extract from "Copying blob sha256:fe07684b16b82247c3539ed86a65ff37a76138ec25d380bd80c869a1a4c73236"
    if let Some(start) = blob_line.find("sha256:") {
        let hash_part = &blob_line[start..];
        let hash_end = hash_part.find(' ').unwrap_or(hash_part.len());
        let id = hash_part[..hash_end].to_string();
        
        // For now, we don't have size info in the blob line, so we'll use 0
        // In a real implementation, this would be extracted from progress events
        Some(BlobInfo {
            id,
            size: 0, // Will be updated from progress events if available
        })
    } else {
        None
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about)]
pub struct Build {
    #[arg(short, long, help = "Image tag to use (defaults to git commit SHA)")]
    tag: Option<String>,
    #[arg(short, long, help = "Override image name (defaults to directory name)")]
    image_name: Option<String>,
    #[arg(long, help = "Path to Dockerfile (defaults to ./Dockerfile)")]
    dockerfile: Option<String>,
    #[arg(long, help = "Build context directory (defaults to current directory)")]
    context: Option<String>,
    #[arg(long, help = "Push the built image to the registry")]
    push: bool,
    #[arg(long, help = "Platform to build for (e.g., linux/amd64)")]
    platform: Option<String>,
    #[arg(short, long, help = "Show raw build output instead of progress bar")]
    verbose: bool,
}

impl Build {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        // Determine build context
        let context_path = self.context.as_deref().unwrap_or(".");
        let dockerfile_path = self.dockerfile.as_deref().unwrap_or("Dockerfile");

        // Check if Dockerfile exists
        let dockerfile_full_path = Path::new(context_path).join(dockerfile_path);
        if !dockerfile_full_path.exists() {
            return Err(anyhow!(
                "Dockerfile not found at {}",
                dockerfile_full_path.display()
            ));
        }

        // Determine if we should skip authentication
        // Skip auth if tag is provided with image name, or if image_name is provided
        let should_skip_auth = (self.tag.is_some() && self.tag.as_ref().unwrap().contains(':')) ||
            self.image_name.is_some();

        // Get image name and tag
        let full_image = if should_skip_auth {
            if let Some(tag) = &self.tag {
                if tag.contains(':') {
                    // Tag contains full image name (e.g., "myapp:v1.0")
                    tag.clone()
                } else {
                    // Tag is just the tag part, need to combine with image name
                    let image_name = self.image_name.unwrap_or_else(|| {
                        let cur_dir = env::current_dir().expect("Unable to get current directory");
                        cur_dir.file_name()
                            .expect("Unable to get directory name")
                            .to_str()
                            .expect("Directory name is not valid UTF-8")
                            .to_string()
                    });
                    format!("{}:{}", image_name, tag)
                }
            } else {
                // No tag provided, use image name with default tag
                let image_name = self.image_name.unwrap_or_else(|| {
                    let cur_dir = env::current_dir().expect("Unable to get current directory");
                    cur_dir.file_name()
                        .expect("Unable to get directory name")
                        .to_str()
                        .expect("Directory name is not valid UTF-8")
                        .to_string()
                });
                let tag = get_image_tag(&self.tag)?;
                format!("{}:{}", image_name, tag)
            }
        } else {
            // Use API to get the proper image name
            let token = base
                .user_config()
                .get_token()
                .ok_or_else(|| anyhow!("Not logged in. Please run 'molnctl auth login' first."))?;
            let tenant_name = base.get_tenant()?;
            let project_name = base.get_project()?;
            
            let api_client = base.api_client();
            let image_name = get_image_name(
                &api_client,
                &token,
                &tenant_name,
                &project_name,
                &self.image_name,
            )?;
            let tag = get_image_tag(&self.tag)?;
            format!("{}:{}", image_name, tag)
        };

        println!("Building image: {}", full_image);
        println!("Context: {}", context_path);
        println!("Dockerfile: {}", dockerfile_path);
        println!();

        // Start timing the entire build process
        let total_start = Instant::now();

        // Create a new tokio runtime for the async operations
        let runtime = tokio::runtime::Runtime::new()?;

        // Create a progress logger
        let logger = Arc::new(ProgressLogger::new(self.verbose));

        // Variables for the build
        let platform = self.platform.as_deref().unwrap_or(DEFAULT_PLATFORM);
        let push = self.push;

        // Execute build and verify
        let (build_duration, verify_duration) = runtime.block_on(async {
            let build_start = Instant::now();
            
            // Execute the build
            execute_build(
                context_path,
                dockerfile_path,
                &full_image,
                platform,
                self.verbose,
                &logger,
            ).await?;

            let build_duration = build_start.elapsed();

            // Push to registry if requested
            if push {
                logger.set_message("📤 Pushing to registry...");
                // The image is already tagged, so it should be available for pushing
                // We would need to implement push logic here if needed
                // For now, we'll just note that it's built and available
            }

            // Verify the image was built successfully
            let verify_start = Instant::now();
            logger.set_message("🔍 Verifying image...");

            let verify_result = verify_image(&full_image).await;
            let verify_duration = verify_start.elapsed();

            match verify_result {
                Ok((success_msg, image_size, actual_layer_count)) => {
                    // Update final image size and adjust cache statistics based on actual layer count
                    {
                        let mut stats = logger.build_stats.lock().unwrap();
                        stats.total_image_size = image_size;
                        
                        // Calculate how many layers were actually processed vs total layers in image
                        let processed_layers = stats.cache_hits + stats.cache_misses;
                        
                        if actual_layer_count > processed_layers {
                            // We have more layers in the final image than we counted during build
                            // This means some base image layers were already cached and not counted
                            let uncounted_base_layers = actual_layer_count - processed_layers;
                            stats.base_image_layers = uncounted_base_layers;
                            stats.cache_hits += uncounted_base_layers;
                            stats.layers_processed += uncounted_base_layers;
                        }
                    }
                    logger.set_message(&success_msg);
                }
                Err(e) => {
                    logger.set_message(&format!("⚠️ Verification failed: {}", e));
                }
            }

            Ok::<(Duration, Duration), anyhow::Error>((build_duration, verify_duration))
        })?;

        let total_duration = total_start.elapsed();

        // Format timing statistics
        fn format_duration(d: Duration) -> String {
            let secs = d.as_secs_f64();
            if secs >= 60.0 {
                format!("{:.1}m", secs / 60.0)
            } else if secs >= 1.0 {
                format!("{:.1}s", secs)
            } else {
                format!("{}ms", d.as_millis())
            }
        }

        // Final output
        let push_info = if push { " 📤 Pushed to registry" } else { "" };
        
        let stats = format!(
            "⏱️ {}total (🏗️ {}build + 🔍 {}verify){}",
            format_duration(total_duration),
            format_duration(build_duration),
            format_duration(verify_duration),
            push_info
        );
        
        logger.finish(&format!("✅ Build completed!\n{}", stats));

        Ok(())
    }
}

async fn execute_build(
    context_path: &str,
    dockerfile_path: &str,
    full_image: &str,
    platform: &str,
    verbose: bool,
    logger: &ProgressLogger,
) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    // Create build context tar archive
    let build_context = create_build_context(context_path, dockerfile_path)?;

    // Configure BuildKit build options
    let session_id = format!("molnctl-build-{}", 
        std::process::id().to_string() + &chrono::Utc::now().timestamp().to_string());
    
    let build_image_options = BuildImageOptions {
        t: Some(full_image.to_string()),
        dockerfile: dockerfile_path.to_string(),
        platform: platform.to_string(),
        version: BuilderVersion::BuilderBuildKit,
        session: Some(session_id),
        pull: Some("1".to_string()),
        nocache: false,
        ..Default::default()
    };

    // Start the build stream
    let bytes_body = bytes::Bytes::from(build_context);
    let http_body = http_body_util::Full::new(bytes_body);
    let either_body = http_body_util::Either::Left(http_body);
    let mut build_stream = docker.build_image(
        build_image_options,
        None,
        Some(either_body),
    );

    // Process build events
    while let Some(build_result) = build_stream.next().await {
        match build_result {
            Ok(BuildInfo { 
                stream: Some(output), 
                .. 
            }) => {
                if verbose {
                    logger.handle_build_output(&output);
                } else {
                    logger.parse_and_display_build_output(&output);
                }
            }
            Ok(BuildInfo { 
                error: Some(error), 
                .. 
            }) => {
                logger.handle_build_error(&error);
                return Err(anyhow!("Build failed: {}", error));
            }
            Ok(BuildInfo { 
                aux: Some(buildkit_event), 
                .. 
            }) => {
                // Handle BuildKit-specific events
                if verbose {
                    logger.handle_build_output(&format!("BuildKit event: {:?}", buildkit_event));
                } else {
                    logger.handle_buildkit_event(&buildkit_event);
                }
            }
            Ok(BuildInfo { 
                status: Some(status), 
                .. 
            }) => {
                logger.handle_build_output(&format!("📊 Status: {}", status));
            }
            Ok(BuildInfo { 
                progress: Some(progress), 
                .. 
            }) => {
                logger.handle_build_output(&format!("📈 Progress: {}", progress));
            }
            Ok(BuildInfo { 
                id: Some(id), 
                .. 
            }) => {
                logger.handle_build_output(&format!("🆔 ID: {}", &id[..SHA_DISPLAY_LENGTH.min(id.len())]));
            }
            Ok(info) => {
                // Handle other BuildInfo types
                if verbose {
                    logger.handle_build_output(&format!("📋 BuildInfo: {:?}", info));
                }
            }
            Err(e) => {
                logger.handle_build_error(&format!("Stream error: {}", e));
                return Err(anyhow!("Build stream error: {}", e));
            }
        }
    }

    Ok(())
}

async fn verify_image(full_image: &str) -> Result<(String, u64, u32)> {
    let docker = Docker::connect_with_local_defaults()?;

    // List images to find our image
    let mut filters = std::collections::HashMap::new();
    filters.insert("reference".to_string(), vec![full_image.to_string()]);
    let list_options = ListImagesOptions {
        filters: Some(filters),
        ..Default::default()
    };

    let images = docker.list_images(Some(list_options)).await?;

    if let Some(image) = images.first() {
        let image_size = image.size as u64;
        
        // Inspect the image to get layer count
        let inspect_result = docker.inspect_image(full_image).await?;
        let layer_count = inspect_result.root_fs.as_ref()
            .and_then(|fs| fs.layers.as_ref())
            .map(|layers| layers.len() as u32)
            .unwrap_or(0);
        
        // Check repo_tags field
        if let Some(tag) = image.repo_tags.first() {
            Ok((format!("📊 {} ({}) • {} layers", tag, format_bytes(image_size), layer_count), image_size, layer_count))
        } else {
            Ok(("✅ Image verified successfully".to_string(), image_size, layer_count))
        }
    } else {
        Err(anyhow!("Could not verify image in Docker daemon"))
    }
}

fn get_image_name(
    api_client: &APIClient,
    token: &str,
    tenant_name: &str,
    project_name: &str,
    name: &Option<String>,
) -> Result<String> {
    let image_name = if let Some(name) = name {
        name.clone()
    } else {
        // Default: use current directory name
        let cur_dir = env::current_dir()?;
        let image_name = if let Some(dir_name) = cur_dir.file_name() {
            dir_name.to_str().unwrap()
        } else {
            return Err(anyhow!("Unable to get current directory for image name"));
        };
        image_name.to_string()
    };
    // Get project ID from API
    let project_id = api_client.get_project(token, tenant_name, project_name)?.id;

    // Format: oci.se-ume.mltt.art/{project_id}/{image_name}
    Ok(format!("oci.se-ume.mltt.art/{}/{}", project_id, image_name))
}

fn get_image_tag(tag: &Option<String>) -> Result<String> {
    if let Some(tag) = tag {
        Ok(tag.clone())
    } else {
        // Default: use git commit SHA (short version)
        let git_output = Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("HEAD")
            .output()?;

        if !git_output.status.success() {
            return Err(anyhow!(
                "Failed to get git commit SHA. Make sure you're in a git repository."
            ));
        }

        Ok(String::from_utf8_lossy(&git_output.stdout)
            .trim()
            .to_string())
    }
}

fn get_ignore_patterns(context_path: &str) -> Result<Vec<String>> {
    let mut patterns = Vec::new();

    // Read .dockerignore first (it takes precedence for Docker builds)
    let dockerignore_path = Path::new(context_path).join(".dockerignore");
    if dockerignore_path.exists() {
        let content = fs::read_to_string(&dockerignore_path)?;
        patterns.extend(parse_ignore_file(&content));
    } else {
        // Fall back to .gitignore if .dockerignore doesn't exist
        let gitignore_path = Path::new(context_path).join(".gitignore");
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            patterns.extend(parse_ignore_file(&content));
        }
    }

    // Add some common patterns that should always be excluded from Docker builds
    patterns.extend(vec![
        ".git".to_string(),
        ".git/**".to_string(),
        "**/.git/**".to_string(),
        ".dockerignore".to_string(),
    ]);

    Ok(patterns)
}

fn parse_ignore_file(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            // Convert gitignore patterns to Dagger-compatible patterns
            if line.starts_with('/') {
                // Absolute path from root
                line[1..].to_string()
            } else if line.ends_with('/') {
                // Directory pattern
                format!("{}**", line)
            } else {
                // File or directory pattern
                line.to_string()
            }
        })
        .collect()
}

fn create_build_context(context_path: &str, _dockerfile_path: &str) -> Result<Vec<u8>> {
    // Create a temporary buffer for the tar archive
    let mut tar_buffer = Vec::new();
    
    // Get ignore patterns for the build context
    let ignore_patterns = get_ignore_patterns(context_path).unwrap_or_default();
    
    // Create tar archive builder and add files
    {
        let mut tar_builder = tar::Builder::new(&mut tar_buffer);
        add_directory_to_tar(&mut tar_builder, context_path, "", &ignore_patterns)?;
        tar_builder.finish()?;
    }
    
    Ok(tar_buffer)
}

fn add_directory_to_tar(
    tar_builder: &mut tar::Builder<&mut Vec<u8>>,
    dir_path: &str,
    tar_prefix: &str,
    ignore_patterns: &[String],
) -> Result<()> {
    let dir = fs::read_dir(dir_path)?;
    
    for entry in dir {
        let entry = entry?;
        let file_path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        
        // Check if this file/directory should be ignored
        let relative_path = if tar_prefix.is_empty() {
            file_name_str.to_string()
        } else {
            format!("{}/{}", tar_prefix, file_name_str)
        };
        
        if should_ignore(&relative_path, ignore_patterns) {
            continue;
        }
        
        if file_path.is_dir() {
            // Recursively add directory contents
            add_directory_to_tar(
                tar_builder,
                &file_path.to_string_lossy(),
                &relative_path,
                ignore_patterns,
            )?;
        } else {
            // Add file to tar
            let mut file = fs::File::open(&file_path)?;
            tar_builder.append_file(&relative_path, &mut file)?;
        }
    }
    
    Ok(())
}

fn should_ignore(path: &str, ignore_patterns: &[String]) -> bool {
    for pattern in ignore_patterns {
        if matches_ignore_pattern(path, pattern) {
            return true;
        }
    }
    false
}

fn matches_ignore_pattern(path: &str, pattern: &str) -> bool {
    // Simple pattern matching - can be enhanced with proper glob matching
    if pattern.contains("**") {
        // Handle ** patterns (matches any number of directories)
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');
            
            return path.starts_with(prefix) && path.ends_with(suffix);
        }
    }
    
    if pattern.contains('*') {
        // Handle single * patterns
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }
    
    // Exact match or directory match
    path == pattern || path.starts_with(&format!("{}/", pattern))
}
