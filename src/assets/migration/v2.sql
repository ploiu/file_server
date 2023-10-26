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
    foreign key (fileRecordId) references FileRecords (id),
    foreign key (tagId) references Tags (id),
    unique (fileRecordId, tagId)
);

create table Folders_Tags
(
    foldersTagId integer primary key autoincrement,
    folderId     integer,
    tagId        integer,
    foreign key (folderId) references Folders (id),
    foreign key (tagId) references Tags (id),
    unique (folderId, tagId)
);

update Metadata
set value = '2'
where name = 'version';

commit;
