begin;
-- introspective data about the database
create table Metadata
(
    id    integer primary key autoincrement,
    name  varchar(128) not null unique,
    value varchar(64)  not null
);

insert into Metadata(name, value)
values ('version', '1');


create table Folders
(
    id       integer primary key autoincrement,
    name     varchar(100),
    parentId integer,
    foreign key (parentId) references Folders (id)
);

-- data about each uploaded file
create table FileRecords
(
    id   integer primary key autoincrement,
    name varchar(256) not null,
    hash char(32)     not null unique
);

create table Folder_Files (
    id integer primary key autoincrement,
    folderId integer not null,
    fileId integer not null unique,
    foreign key(folderId) references Folders(id),
    foreign key(fileId) references FileRecords(id)
);

commit;