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

-- data about each uploaded file
create table FileRecords
(
    id   integer primary key autoincrement,
    name varchar(256) not null,
    path varchar(512) not null unique,
    hash char(32)     not null unique
);
commit;