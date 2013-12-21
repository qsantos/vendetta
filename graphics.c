#include "graphics.h"

#include "util.h"

graphics_t* graphics_init(void)
{
	graphics_t* g = CALLOC(graphics_t, 1);

	sfVideoMode mode = {800, 600, 32};
	g->render = sfRenderWindow_create(mode, "Vendetta 0.2", sfResize | sfClose, NULL);
	if (g->render == NULL)
		exit(1);

	return g;
}

void graphics_exit(graphics_t* g)
{
	sfRenderWindow_destroy(g->render);
	free(g);
}
