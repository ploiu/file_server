begin;

drop table FilePreviews;

update
    Metadata
set
    value = '5'
where
    name = 'version';

commit;