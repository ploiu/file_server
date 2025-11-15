select
    ti.fileId,
    t.id,
    t.title
from
    TaggedItems ti
    join Tags t on ti.tagId = t.id
where
    ti.fileId in ({ })