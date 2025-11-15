begin;

-- Add isDirect column to Files_Tags to distinguish direct vs inherited tags
alter table Files_Tags
add column isDirect integer not null default 1;

-- Add isDirect column to Folders_Tags to distinguish direct vs inherited tags
alter table Folders_Tags
add column isDirect integer not null default 1;

update Metadata
set value = '6'
where name = 'version';

commit;
