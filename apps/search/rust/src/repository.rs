use crate::models::{Candidate, Engagement, Label};
use sea_query::{
    Alias, CommonTableExpression, Condition, Expr, ExprTrait, Func, JoinType, Order,
    PostgresQueryBuilder, Query, WithClause, WithQuery, extension::postgres::PgExpr,
};
use sea_query_sqlx::SqlxBinder;
use sqlx::{FromRow, PgPool};

// SQLx maps the selected database columns into this struct.
#[derive(Debug, FromRow)]
struct CandidateRow {
    id: i64,
    title: String,
    description: String,
    topic_labels: Option<serde_json::Value>, // option: might be null
    created_at: chrono::DateTime<chrono::Utc>,
    likes: i32,
    comment_count: i64, // calculated using sql join below
    view_count: i64,    // calculated using sql join below
}

// Build one statement. CTEs are named query results, not permanent tables.
fn candidate_query(filter: Condition, limit: u64) -> WithQuery {
    // build the primary CTE for the actual posts
    // start the query on the main 'home_thread' table
    // filter is the search words and tags
    let candidates = Query::select()
        .columns([
            "id",
            "title",
            "description",
            "topic_labels",
            "created_at",
            "likes",
        ])
        .from("home_thread")
        .cond_where(filter)
        .order_by("created_at", Order::Desc)
        .order_by("id", Order::Desc)
        .limit(limit)
        .to_owned();

    // turning the fragment above into a formal CTE
    let candidates_cte = CommonTableExpression::new()
        .table_name(Alias::new("candidates"))
        .query(candidates)
        .materialized(true) // reuse candidates within this statement
        .to_owned(); // take ownership of the memory

    // building a reusable function to count engagements (comments/views a post has)
    // we only want to count comments/views for the posts we just found above not all the db
    let count_cte = |table: &'static str, name: &'static str| {
        let counts = Query::select() // build an aggregate SELECT
            .column((Alias::new(table), Alias::new("thread_id"))) // SELECT the thread_id
            .expr_as(
                // SELECT COUNT(thread_id) AS count
                Expr::col((Alias::new(table), Alias::new("thread_id"))).count(),
                Alias::new("count"),
            )
            .from(Alias::new(table))
            .join(
                JoinType::InnerJoin,
                Alias::new("candidates"),
                Expr::col((Alias::new("candidates"), Alias::new("id")))
                    .equals((Alias::new(table), Alias::new("thread_id"))),
            )
            .group_by_col((Alias::new(table), Alias::new("thread_id")))
            .to_owned();
        // package this sub query into its own CTE
        CommonTableExpression::new()
            .table_name(Alias::new(name))
            .query(counts)
            .to_owned()
    };

    // assemble all 3 ctes together into a with clause (WITH candidates AS ....)
    let with = WithClause::new()
        .cte(candidates_cte)
        .cte(count_cte("home_comment", "comment_counts"))
        .cte(count_cte("home_threadview", "view_counts"))
        .to_owned();

    // Join the three named query results.
    let mut query = Query::select();
    query.from(Alias::new("candidates")); // SELECT ... FROM
    for column in [
        "id",
        "title",
        "description",
        "topic_labels",
        "created_at",
        "likes",
    ] {
        query.column((Alias::new("candidates"), Alias::new(column)));
    }
    // loop to join the 2 counting CTEs
    for (table, output) in [
        ("comment_counts", "comment_count"),
        ("view_counts", "view_count"),
    ] {
        query
            .expr_as(
                Func::coalesce([
                    // Coalesce: return 0 of none, rather than Null
                    Expr::col((Alias::new(table), Alias::new("count"))),
                    Expr::val(0_i64), // default to 0
                ]),
                Alias::new(output), // output as 'comment_count' or 'view_count'
            )
            .join(
                // LEFT joinso we don't delete posts that have 0 coments
                JoinType::LeftJoin,
                Alias::new(table),
                // ON counts.thread_id = candidates.id
                Expr::col((Alias::new(table), Alias::new("thread_id")))
                    .equals((Alias::new("candidates"), Alias::new("id"))),
            );
    }
    // Return candidate data without imposing a relevance order.
    query.with(with)
}

fn candidate_filter(terms: &[String], labels: &[Label]) -> Option<Condition> {
    let mut conditions = Condition::any();
    let mut has_condition = false;
    for term in terms {
        // terms come from tokenize(), so contain no LIKE wildcard characters.
        let pattern = format!("%{term}%");
        conditions = conditions
            .add(Expr::col("title").ilike(pattern.clone()))
            .add(Expr::col("description").ilike(pattern));
        has_condition = true;
    }
    for label in labels.iter().filter(|label| label.score > 0.0) {
        conditions = conditions.add(
            Expr::col("topic_labels").contains(Expr::val(serde_json::json!([{ "id": label.id }]))),
        );
        has_condition = true;
    }
    has_condition.then_some(conditions)
}

pub async fn fetch_candidates(
    db: &PgPool,
    terms: &[String],
    labels: &[Label],
    limit: u64,
) -> Result<Vec<Candidate>, sqlx::Error> {
    let Some(filter) = candidate_filter(terms, labels) else {
        return Ok(Vec::new());
    };
    let (sql, values) = candidate_query(filter, limit).build_sqlx(PostgresQueryBuilder);
    // SQL comes only from SeaQuery; user values are bound separately.
    let posts =
        sqlx::query_as_with::<_, CandidateRow, _>(sqlx::AssertSqlSafe(sql.as_str()), values)
            .fetch_all(db)
            .await?;

    posts
        .into_iter()
        .map(|post| {
            let labels: Vec<Label> = match post.topic_labels {
                None | Some(serde_json::Value::Null) => Vec::new(),
                Some(value) => serde_json::from_value(value).map_err(|_| {
                    sqlx::Error::Decode(format!("Invalid tags on post {}", post.id).into())
                })?,
            };
            if labels.iter().any(|label| !label.is_valid()) {
                return Err(sqlx::Error::Decode(
                    format!("Invalid tag scores on post {}", post.id).into(),
                ));
            }
            Ok(Candidate {
                id: post.id,
                title: post.title,
                description: post.description,
                labels,
                created_at: post.created_at,
                likes: std::cmp::max(i64::from(post.likes), 0),
                engagement: Engagement {
                    comments: post.comment_count,
                    views: post.view_count,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{candidate_filter, candidate_query, fetch_candidates};
    use crate::models::Label;
    use sea_query::PostgresQueryBuilder;

    #[test]
    fn query_binds_input_instead_of_interpolating_it() {
        let term = "quoted'value".to_owned();
        let label = Label {
            id: "tag'value".to_owned(),
            score: 0.8,
        };
        let filter = candidate_filter(&[term], &[label]).unwrap();
        let (sql, values) = candidate_query(filter, 10).build(PostgresQueryBuilder);
        assert!(!sql.contains("quoted'value"));
        assert!(!sql.contains("tag'value"));
        assert!(sql.contains("MATERIALIZED"));
        assert!(sql.contains("LEFT JOIN"));
        assert!(!values.0.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn postgres_candidates_preserve_filters_counts_and_limits() {
        let url = std::env::var("TEST_DATABASE_URL").expect("set TEST_DATABASE_URL");
        // One connection keeps the temporary fixtures isolated from real tables.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::raw_sql(
            r#"
            CREATE TEMP TABLE home_thread (
                id BIGINT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL,
                topic_labels JSONB, created_at TIMESTAMPTZ NOT NULL, likes INTEGER NOT NULL
            );
            CREATE TEMP TABLE home_comment (thread_id BIGINT NOT NULL);
            CREATE TEMP TABLE home_threadview (thread_id BIGINT NOT NULL);
            INSERT INTO home_thread VALUES
                (1, 'Rust guide', '', NULL, '2026-01-01 00:00:00+00', 4),
                (2, 'Other', 'database guide', 'null', '2026-01-02 00:00:00+00', -2),
                (3, 'Tagged', '', '[{"id":"tech","score":0.2}]', '2026-01-03 00:00:00+00', 1),
                (4, 'Unrelated', '', NULL, '2026-01-04 00:00:00+00', 0);
            INSERT INTO home_comment VALUES (1),(1),(4);
            INSERT INTO home_threadview VALUES (1),(1),(1),(4);
        "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let terms = vec!["rust".to_owned(), "database".to_owned()];
        let labels = vec![Label {
            id: "tech".to_owned(),
            score: 0.8,
        }];
        let mut posts = fetch_candidates(&pool, &terms, &labels, 10).await.unwrap();
        posts.sort_by_key(|p| p.id);
        assert_eq!(
            posts.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(posts[0].engagement.comments, 2);
        assert_eq!(posts[0].engagement.views, 3);
        assert_eq!(posts[1].engagement.comments, 0);
        assert_eq!(posts[1].engagement.views, 0);
        assert_eq!(posts[1].likes, 0);
        assert!(posts[0].labels.is_empty());
        assert!(posts[1].labels.is_empty());
        assert_eq!(posts[2].labels[0].score, 0.2);
        let newest = fetch_candidates(&pool, &terms, &labels, 1).await.unwrap();
        assert_eq!(newest[0].id, 3);
        assert!(
            fetch_candidates(&pool, &[], &[], 10)
                .await
                .unwrap()
                .is_empty()
        );
        let tags_only = fetch_candidates(&pool, &[], &labels, 10).await.unwrap();
        assert_eq!(tags_only[0].id, 3);
        let zero_labels = [Label {
            id: "tech".to_owned(),
            score: 0.0,
        }];
        assert!(
            fetch_candidates(&pool, &[], &zero_labels, 10)
                .await
                .unwrap()
                .is_empty()
        );

        sqlx::raw_sql("UPDATE pg_temp.home_thread SET topic_labels = '[{\"id\":\"tech\",\"score\":2}]' WHERE id = 1")
            .execute(&pool).await.unwrap();
        assert!(fetch_candidates(&pool, &terms, &[], 10).await.is_err());
        pool.close().await;
    }
}
