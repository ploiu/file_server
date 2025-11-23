-- Remove implicit tags from folders where the tag is inherited from a specific folder
delete from
  TaggedItems
where
  tagId = ?1
  and implicitFromId = ?2