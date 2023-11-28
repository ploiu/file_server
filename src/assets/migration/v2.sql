begin;

create table Tags
(
    id    integer primary key autoincrement,
    title varchar not null unique
);

create table Files_Tags
(
    fileRecordTagId integer primary key autoincrement,
    fileRecordId    integer,
    tagId           integer,
    foreign key (fileRecordId) references FileRecords (id) on delete cascade,
    foreign key (tagId) references Tags (id) on delete cascade,
    unique (fileRecordId, tagId)
);

create table Folders_Tags
(
    foldersTagId integer primary key autoincrement,
    folderId     integer,
    tagId        integer,
    foreign key (folderId) references Folders (id) on delete cascade,
    foreign key (tagId) references Tags (id) on delete cascade,
    unique (folderId, tagId)
);

update Metadata
set value = '2'
where name = 'version';

commit;
