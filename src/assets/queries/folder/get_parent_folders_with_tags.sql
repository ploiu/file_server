with recursive query(id) as (
  values
    (?1)
  union
  select
    parentId
  from
    Folders,
    query
  where
    Folders.id = query.id
)
select
  parentId,
  group_concat(t.title)
from
  Folders
  left join TaggedItems ti on ti.folderId = folders.parentId
  left join tags t on t.id = ti.tagId
where
  Folders.parentId in query
  and parentId <> ?1
  and t.title in (?2)
group by
  parentId;