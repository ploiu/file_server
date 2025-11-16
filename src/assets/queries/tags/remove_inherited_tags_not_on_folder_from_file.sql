DELETE FROM TaggedItems 
WHERE fileId = ? 
  AND inheritedFromId = ? 
  AND tagId NOT IN (
    SELECT tagId FROM TaggedItems 
    WHERE folderId = ? 
      AND inheritedFromId IS NULL
  )
