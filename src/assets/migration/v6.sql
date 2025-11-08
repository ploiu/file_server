begin;

update
    Metadata
set
    value = '6'
where
    name = 'version';

commit;
