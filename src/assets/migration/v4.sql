begin;

create table FileTypes (
    id integer primary key autoincrement,
    fileType text not null unique
);

alter table
    FileRecords
add
    column fileSize integer default 0;

alter table
    FileRecords
add
    column dateCreated datetime default current_timestamp;

create table FileRecordTypes (
    id integer primary key autoincrement,
    fildId integer references FileRecords(id) on delete cascade,
    fileTypeId integer references FileTypes(id)
);

insert into
    FileTypes(fileType)
values
    ("application"),
    ("archive"),
    ("audio"),
    ("cad"),
    ("calendar"),
    ("code"),
    ("configuration"),
    ("diagram"),
    ("document"),
    ("font"),
    ("game_rom"),
    ("image"),
    ("material"),
    ("model"),
    ("object"),
    ("presentation"),
    ("save_file"),
    ("spreadsheet"),
    ("text"),
    ("video");

update
    Metadata
set
    value = '4'
where
    name = 'version';

commit;