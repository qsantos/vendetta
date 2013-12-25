#ifndef W_OBJECT_H
#define W_OBJECT_H

typedef enum
{
	O_CHARACTER,
	O_MINE,
	O_BUILDING,
} otype_t;

typedef struct object object_t;

struct object
{
	otype_t t;

	float x;
	float y;
	float w;
	float h;
};

int object_isAt(object_t* o, float x, float y);
int object_rect(object_t* o, float x, float y, float w, float h);

#endif
