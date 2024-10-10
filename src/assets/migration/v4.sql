begin;

create table FileTypes (
    id integer primary key autoincrement,
    value text not null unique
);

alter table
    FileRecords
add
    column fileSize integer default 0;

alter table
    FileRecords
add
    column dateCreated datetime;
-- we can't call a function for a default value when altering a table, which is why we need the update below.
update
    FileRecords
set
    dateCreated = datetime(CURRENT_TIMESTAMP, 'localtime');

create table FileRecordTypes (
    id integer primary key autoincrement,
    fileId integer references FileRecords(id) on delete cascade,
    fileTypeId integer references FileTypes(id)
);

insert into
    FileTypes(value)
values
    ('application'),
    ('archive'),
    ('audio'),
    ('cad'),
    ('code'),
    ('configuration'),
    ('diagram'),
    ('document'),
    ('font'),
    ('game_rom'),
    ('image'),
    ('material'),
    ('model'),
    ('object'),
    ('presentation'),
    ('save_file'),
    ('spreadsheet'),
    ('text'),
    ('video'),
    ('unknown');

update
    Metadata
set
    value = '4'
where
    name = 'version';

commit;