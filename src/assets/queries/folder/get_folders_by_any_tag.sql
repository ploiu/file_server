select
    f.id,
    f.name,
    f.parentId,
    group_concat(t.title)
from
    folders f
    join TaggedItems ti on ti.folderId = f.id
    join tags t on t.id = ti.tagId
where
    t.title in (?1)
group by
    f.id;