/*
 travels up the ancestor tree of a folder and retrieves their IDs. 
 The depth counter allows us to guarantee that retrieval order goes from closest to ?1 -> furthest from ?1
 */
with recursive ancestors(id, depth) as (
    select
        parentId,
        1
    from
        Folders
    where
        id = ?1
        and parentId is not null
    union
    all
    select
        f.parentId,
        a.depth + 1
    from
        Folders f
        join ancestors a on f.id = a.id
    where
        f.parentId is not null
)
select
    id
from
    ancestors
order by
    depth asc;