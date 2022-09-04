begin;
-- introspective data about the database
create table Metadata
(
    id    integer primary key autoincrement,
    name  varchar not null unique,
    value varchar not null
);

insert into Metadata(name, value)
values ('version', '1');


create table Folders
(
    id       integer primary key autoincrement,
    name     varchar,
    parentId integer,
    foreign key (parentId) references Folders (id)
);

-- data about each uploaded file
create table FileRecords
(
    id   integer primary key autoincrement,
    name varchar not null
);

create table Folder_Files
(
    id       integer primary key autoincrement,
    folderId integer not null,
    fileId   integer not null,
    foreign key (folderId) references Folders (id),
    foreign key (fileId) references FileRecords (id) on delete cascade
);

commit;