use std::{fs, path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};
use perwiga_core::{
    feed::{normalize_feed_items, parse_feed},
    http::{fetch_and_normalize, HttpFeedTransport},
    model::{AliasInput, EntityInput, EntityPatch},
    service::Application,
    ModuleRegistry, Store, WorkKind,
};

mod public_export;

#[derive(Debug, Parser)]
#[command(name = "perwiga", about = "Local-first game and novel wiki")]
struct Cli {
    /// SQLite path. Use a temporary path for experiments and tests.
    #[arg(long, global = true, default_value = "perwiga.sqlite")]
    database: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Modules,
    ExportPublic(public_export::ExportPublicCommand),
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },
    Entity {
        #[command(subcommand)]
        command: EntityCommand,
    },
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },
    Folder(FolderCommand),
    FolderImages(FolderImagesCommand),
    Checklist {
        #[command(subcommand)]
        command: ChecklistCommand,
    },
    Feed {
        #[command(subcommand)]
        command: FeedCommand,
    },
    Event(EventCommand),
}

#[derive(Debug, Subcommand)]
enum WorkCommand {
    Add(AddWork),
    List,
}

#[derive(Debug, Args)]
struct AddWork {
    #[arg(long, value_parser = parse_kind)]
    kind: WorkKind,
    #[arg(long)]
    module: String,
    #[arg(long)]
    name: String,
}

#[derive(Debug, Subcommand)]
enum EntityCommand {
    Add(AddEntity),
    List(ListEntities),
    Search(SearchEntities),
    Show(ShowById),
    Update(UpdateEntity),
    Alias(AddAlias),
    ImportEndfieldAliases(ImportEndfieldAliases),
    ImportGenshinCharacters(ImportGenshinCharacters),
    ImportGenshinRegions(ImportGenshinRegions),
    ImportGenshinNpcs(ImportGenshinNpcs),
    ImportGenshinWeapons(ImportGenshinWeapons),
    ImportGenshinSkins(ImportGenshinSkins),
    ImportGenshinArtifacts(ImportGenshinArtifacts),
    ImportGenshinArtifactDomains(ImportGenshinArtifactDomains),
    ImportGenshinEvents(ImportGenshinEvents),
}

#[derive(Debug, Args)]
struct AddEntity {
    #[arg(long)]
    work: String,
    #[arg(long = "type")]
    entity_type: String,
    #[arg(long)]
    english: String,
    #[arg(long)]
    original: String,
    #[arg(long)]
    vietnamese: Option<String>,
    #[arg(long)]
    automatic_vietnamese: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    other: Option<String>,
}

#[derive(Debug, Args)]
struct ListEntities {
    #[arg(long)]
    work: String,
    #[arg(long = "type")]
    entity_type: Option<String>,
}

#[derive(Debug, Args)]
struct SearchEntities {
    #[arg(long)]
    work: String,
    #[arg(long)]
    query: String,
}

#[derive(Debug, Args)]
struct ShowById {
    #[arg(long)]
    id: String,
}

#[derive(Debug, Args)]
struct UpdateEntity {
    #[arg(long)]
    id: String,
    #[arg(long)]
    english: Option<String>,
    #[arg(long)]
    original: Option<String>,
    #[arg(long)]
    vietnamese: Option<String>,
    #[arg(long)]
    automatic_vietnamese: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    other: Option<String>,
}

#[derive(Debug, Args)]
struct AddAlias {
    #[arg(long)]
    entity: String,
    #[arg(long)]
    value: String,
    #[arg(long)]
    kind: String,
    #[arg(long)]
    language: Option<String>,
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Debug, Args)]
struct ImportEndfieldAliases {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinCharacters {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinRegions {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinNpcs {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinWeapons {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinSkins {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinArtifacts {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinArtifactDomains {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Args)]
struct ImportGenshinEvents {
    #[arg(long)]
    work: String,
}

#[derive(Debug, Subcommand)]
enum NoteCommand {
    Add(AddNote),
    Update(UpdateNote),
    Attach(AttachNote),
    List {
        #[arg(long)]
        work: String,
    },
}

#[derive(Debug, Args)]
struct UpdateNote {
    #[arg(long)]
    id: String,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    content: Option<String>,
}

#[derive(Debug, Args)]
struct AddNote {
    #[arg(long)]
    work: String,
    #[arg(long)]
    title: String,
    /// Safe plain text or Markdown-like content. It is stored as text and never executed as markup.
    #[arg(long)]
    content: String,
}

#[derive(Debug, Args)]
struct AttachNote {
    #[arg(long)]
    note: String,
    #[arg(long)]
    source_type: String,
    #[arg(long)]
    source: String,
}

#[derive(Debug, Args)]
struct FolderCommand {
    #[arg(long)]
    work: String,
    #[arg(long)]
    path: String,
    #[arg(long)]
    label: Option<String>,
}

#[derive(Debug, Args)]
struct FolderImagesCommand {
    #[arg(long)]
    folder: String,
}

#[derive(Debug, Subcommand)]
enum ChecklistCommand {
    Add {
        #[arg(long)]
        work: String,
        #[arg(long)]
        title: String,
    },
    Item {
        #[arg(long)]
        checklist: String,
        #[arg(long)]
        label: String,
    },
    Toggle {
        #[arg(long)]
        item: String,
    },
}

#[derive(Debug, Subcommand)]
enum FeedCommand {
    Add {
        #[arg(long)]
        work: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        provenance: String,
    },
    Ingest(IngestFeed),
    Refresh(RefreshFeed),
    Read {
        #[arg(long)]
        item: String,
    },
    List {
        #[arg(long)]
        source: String,
    },
}

#[derive(Debug, Args)]
struct IngestFeed {
    #[arg(long)]
    source: String,
    #[arg(long)]
    xml_file: PathBuf,
    #[arg(long)]
    provenance: String,
    #[arg(long)]
    discovered_at: String,
}

#[derive(Debug, Args)]
struct RefreshFeed {
    #[arg(long)]
    source: String,
    #[arg(long)]
    provenance: String,
    #[arg(long)]
    discovered_at: String,
}

#[derive(Debug, Args)]
struct EventCommand {
    #[arg(long)]
    work: String,
    #[arg(long)]
    title: String,
    #[arg(long)]
    starts_at: String,
    #[arg(long)]
    ends_at: Option<String>,
    #[arg(long, default_value_t = false)]
    all_day: bool,
    #[arg(long)]
    source_url: Option<String>,
    #[arg(long, default_value = "manual")]
    provider: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> perwiga_core::Result<()> {
    let cli = Cli::parse();
    let mut registry = ModuleRegistry::default();
    perwiga_game_generic::register(&mut registry)?;
    perwiga_game_arknights_endfield::register(&mut registry)?;
    perwiga_game_genshin_impact::register(&mut registry)?;
    perwiga_novel_generic::register(&mut registry)?;
    let mut app = Application::new(Store::open(&cli.database)?, registry);

    match cli.command {
        Command::Modules => {
            for module in app.modules() {
                println!(
                    "{}:{}\t{}",
                    module.kind(),
                    module.id(),
                    module.display_name()
                );
                for entity_type in module.entity_types() {
                    println!("  {}\t{}", entity_type.key, entity_type.display_name);
                }
            }
        }
        Command::ExportPublic(input) => {
            public_export::write_catalog(app.store(), app.modules(), &input.output)?;
        }
        Command::Work { command } => match command {
            WorkCommand::Add(input) => {
                print_json(&app.create_work(input.kind, &input.module, &input.name)?)?
            }
            WorkCommand::List => print_json(&app.store().list_works()?)?,
        },
        Command::Entity { command } => match command {
            EntityCommand::Add(input) => print_json(&app.create_entity(
                &input.work,
                &EntityInput {
                    entity_type: input.entity_type,
                    official_english_name: input.english,
                    official_original_name: input.original,
                    official_vietnamese_name: input.vietnamese,
                    automatic_vietnamese_translation: input.automatic_vietnamese,
                    english_description: input.description,
                    other_information: input.other,
                },
            )?)?,
            EntityCommand::List(input) => print_json(
                &app.store()
                    .list_entities(&input.work, input.entity_type.as_deref())?,
            )?,
            EntityCommand::Search(input) => {
                print_json(&app.store().search_entities(&input.work, &input.query)?)?
            }
            EntityCommand::Show(input) => {
                print_json(&app.store().get_entity_detail(&input.id)?.ok_or_else(|| {
                    perwiga_core::PerwigaError::NotFound(format!("entity {}", input.id))
                })?)?
            }
            EntityCommand::Update(input) => print_json(&app.update_entity(
                &input.id,
                &EntityPatch {
                    official_english_name: input.english,
                    official_original_name: input.original,
                    official_vietnamese_name: input.vietnamese,
                    automatic_vietnamese_translation: input.automatic_vietnamese,
                    english_description: input.description,
                    other_information: input.other,
                },
            )?)?,
            EntityCommand::Alias(input) => print_json(&app.add_alias(
                &input.entity,
                &AliasInput {
                    value: input.value,
                    language: input.language,
                    kind: input.kind,
                    label: input.label,
                    notes: input.notes,
                },
            )?)?,
            EntityCommand::ImportEndfieldAliases(input) => {
                print_json(&perwiga_game_arknights_endfield::import_curated_aliases(
                    app.store_mut(),
                    &input.work,
                )?)?
            }
            EntityCommand::ImportGenshinCharacters(input) => {
                print_json(&perwiga_game_genshin_impact::import_curated_characters(
                    app.store_mut(),
                    &input.work,
                )?)?
            }
            EntityCommand::ImportGenshinRegions(input) => print_json(
                &perwiga_game_genshin_impact::import_curated_regions(app.store_mut(), &input.work)?,
            )?,
            EntityCommand::ImportGenshinNpcs(input) => print_json(
                &perwiga_game_genshin_impact::import_curated_npcs(app.store_mut(), &input.work)?,
            )?,
            EntityCommand::ImportGenshinWeapons(input) => print_json(
                &perwiga_game_genshin_impact::import_curated_weapons(app.store_mut(), &input.work)?,
            )?,
            EntityCommand::ImportGenshinSkins(input) => print_json(
                &perwiga_game_genshin_impact::import_curated_skins(app.store_mut(), &input.work)?,
            )?,
            EntityCommand::ImportGenshinArtifacts(input) => {
                print_json(&perwiga_game_genshin_impact::import_curated_artifacts(
                    app.store_mut(),
                    &input.work,
                )?)?
            }
            EntityCommand::ImportGenshinArtifactDomains(input) => print_json(
                &perwiga_game_genshin_impact::import_curated_artifact_domains(
                    app.store_mut(),
                    &input.work,
                )?,
            )?,
            EntityCommand::ImportGenshinEvents(input) => {
                let entities = perwiga_game_genshin_impact::import_curated_event_entities(
                    app.store_mut(),
                    &input.work,
                )?;
                let events = perwiga_game_genshin_impact::curated_calendar_events()?;
                let calendar = app
                    .store_mut()
                    .import_calendar_events(&input.work, &events)?;
                print_json(&serde_json::json!({
                    "entities": entities,
                    "calendar_events": {
                        "inserted": calendar.inserted,
                        "unchanged": calendar.unchanged,
                    },
                }))?
            }
        },
        Command::Note { command } => match command {
            NoteCommand::Add(input) => print_json(&app.store().create_note(
                &input.work,
                &input.title,
                &input.content,
            )?)?,
            NoteCommand::Update(input) => print_json(&app.store().update_note(
                &input.id,
                input.title.as_deref(),
                input.content.as_deref(),
            )?)?,
            NoteCommand::Attach(input) => print_json(&app.store().attach_to_note(
                &input.note,
                &input.source_type,
                &input.source,
            )?)?,
            NoteCommand::List { work } => print_json(&app.store().list_notes(&work)?)?,
        },
        Command::Folder(input) => print_json(&app.store().link_folder(
            &input.work,
            &input.path,
            input.label.as_deref(),
        )?)?,
        Command::FolderImages(input) => {
            print_json(&app.store().list_folder_images(&input.folder)?)?
        }
        Command::Checklist { command } => match command {
            ChecklistCommand::Add { work, title } => {
                print_json(&app.store().create_checklist(&work, &title)?)?
            }
            ChecklistCommand::Item { checklist, label } => {
                print_json(&app.store().add_checklist_item(&checklist, &label)?)?
            }
            ChecklistCommand::Toggle { item } => {
                print_json(&app.store().toggle_checklist_item(&item)?)?
            }
        },
        Command::Feed { command } => match command {
            FeedCommand::Add {
                work,
                url,
                provenance,
            } => print_json(&app.create_feed_source(&work, &url, &provenance)?)?,
            FeedCommand::Ingest(input) => ingest_feed(&mut app, input)?,
            FeedCommand::Refresh(input) => refresh_feed(&mut app, input)?,
            FeedCommand::Read { item } => {
                print_json(&app.store().set_feed_item_read(&item, true)?)?
            }
            FeedCommand::List { source } => print_json(&app.store().list_feed_items(&source)?)?,
        },
        Command::Event(input) => print_json(&app.store().create_calendar_event(
            &input.work,
            &input.title,
            &input.starts_at,
            input.ends_at.as_deref(),
            input.all_day,
            input.source_url.as_deref(),
            &input.provider,
        )?)?,
    }
    Ok(())
}

fn ingest_feed(app: &mut Application, input: IngestFeed) -> perwiga_core::Result<()> {
    let xml = fs::read_to_string(&input.xml_file).map_err(|error| {
        perwiga_core::PerwigaError::Validation(format!("cannot read feed fixture: {error}"))
    })?;
    let parsed = match parse_feed(&xml) {
        Ok(parsed) => parsed,
        Err(error) => {
            app.store()
                .record_feed_refresh(&input.source, false, Some(&error.to_string()))?;
            return Err(error);
        }
    };
    let normalized = normalize_feed_items(&parsed, &input.discovered_at, &input.provenance)?;
    let items = app
        .store_mut()
        .upsert_feed_items(&input.source, &normalized)?;
    app.store().record_feed_refresh(&input.source, true, None)?;
    print_json(&items)
}

fn refresh_feed(app: &mut Application, input: RefreshFeed) -> perwiga_core::Result<()> {
    let source = app.store().get_feed_source(&input.source)?.ok_or_else(|| {
        perwiga_core::PerwigaError::NotFound(format!("feed source {}", input.source))
    })?;
    let transport = HttpFeedTransport::default();
    let normalized = match fetch_and_normalize(
        &transport,
        &source.url,
        &input.discovered_at,
        &input.provenance,
    ) {
        Ok(items) => items,
        Err(error) => {
            app.store()
                .record_feed_refresh(&source.id, false, Some(&error.to_string()))?;
            return Err(error);
        }
    };
    let items = app.store_mut().upsert_feed_items(&source.id, &normalized)?;
    app.store().record_feed_refresh(&source.id, true, None)?;
    print_json(&items)
}

fn parse_kind(value: &str) -> std::result::Result<WorkKind, String> {
    WorkKind::from_str(value).map_err(|error| error.to_string())
}

fn print_json<T: serde::Serialize>(value: &T) -> perwiga_core::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| perwiga_core::PerwigaError::Validation(error.to_string()))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endfield_curated_alias_import() {
        let cli = Cli::try_parse_from([
            "perwiga",
            "entity",
            "import-endfield-aliases",
            "--work",
            "work-1",
        ])
        .expect("valid curated alias import command");

        match cli.command {
            Command::Entity {
                command: EntityCommand::ImportEndfieldAliases(input),
            } => assert_eq!(input.work, "work-1"),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn parses_genshin_curated_character_import() {
        let cli = Cli::try_parse_from([
            "perwiga",
            "entity",
            "import-genshin-characters",
            "--work",
            "work-2",
        ])
        .expect("valid Genshin character import command");

        match cli.command {
            Command::Entity {
                command: EntityCommand::ImportGenshinCharacters(input),
            } => assert_eq!(input.work, "work-2"),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn parses_genshin_curated_weapon_import() {
        let cli = Cli::try_parse_from([
            "perwiga",
            "entity",
            "import-genshin-weapons",
            "--work",
            "work-3",
        ])
        .expect("valid Genshin weapon import command");

        match cli.command {
            Command::Entity {
                command: EntityCommand::ImportGenshinWeapons(input),
            } => assert_eq!(input.work, "work-3"),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn parses_genshin_curated_region_import() {
        let cli = Cli::try_parse_from([
            "perwiga",
            "entity",
            "import-genshin-regions",
            "--work",
            "work-regions",
        ])
        .expect("valid Genshin Region import command");

        match cli.command {
            Command::Entity {
                command: EntityCommand::ImportGenshinRegions(input),
            } => assert_eq!(input.work, "work-regions"),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn parses_genshin_curated_skin_import() {
        let cli = Cli::try_parse_from([
            "perwiga",
            "entity",
            "import-genshin-skins",
            "--work",
            "work-4",
        ])
        .expect("valid Genshin Skin import command");

        match cli.command {
            Command::Entity {
                command: EntityCommand::ImportGenshinSkins(input),
            } => assert_eq!(input.work, "work-4"),
            command => panic!("unexpected command: {command:?}"),
        }
    }
}
