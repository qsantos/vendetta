#ifndef U_STATUS_H
#define U_STATUS_H

typedef struct kindOf_status kindOf_status_t;

#include <wchar.h>

typedef enum
{
	ST_HEALTH,
	ST_STAMINA,
	ST_MORAL,
	ST_MANA,
	N_STATUSES,
} status_type_t;

struct kindOf_status
{
	wchar_t* name;
};

void kindOf_status_init(kindOf_status_t* s);
void kindOf_status_exit(kindOf_status_t* s);

#endif
