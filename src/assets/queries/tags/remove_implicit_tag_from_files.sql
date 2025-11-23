-- Remove implicit tags from files where the tag is inherited from a specific folder
delete from
  TaggedItems
where
  tagId = ?1
  and implicitFromId = ?2