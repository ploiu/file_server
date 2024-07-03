begin;

create table FilePreviews(
    file_id int primary key references FileRecords(id),
    file_preview BLOB not null
);

update Metadata
set value = '3'
where name = 'version';

commit;
